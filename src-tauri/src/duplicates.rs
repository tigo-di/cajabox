//! Arquivos duplicados na pasta aberta, por nome ou por conteúdo.
//!
//! Por conteúdo, em peneiras para ler o mínimo: tamanho (sem ler nada) → amostra do início e do fim
//! → conteúdo completo (BLAKE3), este só para os que passaram pelas duas primeiras.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use serde::Deserialize;

use crate::scan::FileEntry;

/// Bytes lidos do início e do fim na amostra; arquivos até o dobro disso são lidos inteiros nela.
const SAMPLE: u64 = 64 * 1024;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Name,
    Content,
}

/// Busca por conteúdo: impressões já calculadas e pedido de cancelamento.
#[derive(Default)]
pub struct Search {
    /// Por (caminho, tamanho, modificação, completa?): arquivo inalterado não é relido.
    cache: Mutex<HashMap<(PathBuf, u64, SystemTime, bool), blake3::Hash>>,
    cancel: AtomicBool,
}

impl Search {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Impressão da amostra (`full = false`) ou do arquivo inteiro, usando o cache.
    fn hash(&self, path: &Path, size: u64, full: bool) -> io::Result<blake3::Hash> {
        let modified = fs::metadata(path)?.modified()?;
        let key = (path.to_path_buf(), size, modified, full);
        if let Some(hash) = self.cache.lock().unwrap().get(&key) {
            return Ok(*hash);
        }
        let hash = if full { self.full_hash(path)? } else { sample_hash(path, size)? };
        self.cache.lock().unwrap().insert(key, hash);
        Ok(hash)
    }

    fn full_hash(&self, path: &Path) -> io::Result<blake3::Hash> {
        let mut file = File::open(path)?;
        let mut hasher = blake3::Hasher::new();
        let mut buffer = vec![0; 1024 * 1024];
        loop {
            if self.cancelled() {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelado"));
            }
            match file.read(&mut buffer)? {
                0 => return Ok(hasher.finalize()),
                n => hasher.update(&buffer[..n]),
            };
        }
    }

    /// Grupos de caminhos com o mesmo conteúdo, maiores primeiro. `progress(feitos, total)` a cada
    /// arquivo lido. `None` = cancelado. Arquivos vazios e ilegíveis ficam de fora.
    pub fn by_content(&self, files: &[FileEntry], progress: impl Fn(usize, usize)) -> Option<Vec<Vec<String>>> {
        self.cancel.store(false, Ordering::Relaxed);
        let by_size = groups(files.iter().filter(|f| f.size > 0), |f| f.size);
        let mut total: usize = by_size.iter().map(Vec::len).sum();
        let mut done = 0;

        let mut by_sample = Vec::new();
        for group in by_size {
            let hashed = self.hash_each(&group, false, &mut done, total, &progress)?;
            by_sample.extend(groups(hashed.into_iter(), |(_, hash)| *hash).into_iter());
        }

        let mut result = Vec::new();
        total += by_sample.iter().filter(|g| g[0].0.size > 2 * SAMPLE).map(Vec::len).sum::<usize>();
        for group in by_sample {
            let files: Vec<&FileEntry> = group.into_iter().map(|(f, _)| f).collect();
            if files[0].size <= 2 * SAMPLE {
                result.push(files); // a amostra já era o arquivo inteiro
                continue;
            }
            let hashed = self.hash_each(&files, true, &mut done, total, &progress)?;
            result.extend(groups(hashed.into_iter(), |(_, hash)| *hash).into_iter().map(|g| g.into_iter().map(|(f, _)| f).collect()));
        }
        result.sort_by(|a, b| b[0].size.cmp(&a[0].size).then_with(|| a[0].path.cmp(&b[0].path)));
        Some(result.into_iter().map(paths).collect())
    }

    /// Impressão de cada arquivo (os ilegíveis ficam de fora); `None` = cancelado.
    fn hash_each<'a>(
        &self,
        files: &[&'a FileEntry],
        full: bool,
        done: &mut usize,
        total: usize,
        progress: &impl Fn(usize, usize),
    ) -> Option<Vec<(&'a FileEntry, blake3::Hash)>> {
        let mut hashed = Vec::new();
        for file in files {
            if self.cancelled() {
                return None;
            }
            if let Ok(hash) = self.hash(Path::new(&file.path), file.size, full) {
                hashed.push((*file, hash));
            }
            *done += 1;
            progress(*done, total);
        }
        if self.cancelled() { None } else { Some(hashed) }
    }
}

/// Grupos de caminhos com o mesmo nome (sem diferenciar maiúsculas), em ordem de nome.
pub fn by_name(files: &[FileEntry]) -> Vec<Vec<String>> {
    let mut result = groups(files.iter(), |f| f.name.to_lowercase());
    result.sort_by_key(|g| g[0].name.to_lowercase());
    result.into_iter().map(paths).collect()
}

/// Agrupa por `key`, mantendo só grupos com mais de um item.
fn groups<T, K: std::hash::Hash + Eq>(items: impl Iterator<Item = T>, key: impl Fn(&T) -> K) -> Vec<Vec<T>> {
    let mut map: HashMap<K, Vec<T>> = HashMap::new();
    for item in items {
        map.entry(key(&item)).or_default().push(item);
    }
    map.into_values().filter(|g| g.len() > 1).collect()
}

fn paths(group: Vec<&FileEntry>) -> Vec<String> {
    let mut paths: Vec<String> = group.into_iter().map(|f| f.path.clone()).collect();
    paths.sort();
    paths
}

/// Impressão do início e do fim do arquivo (inteiro, se pequeno).
fn sample_hash(path: &Path, size: u64) -> io::Result<blake3::Hash> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0; SAMPLE as usize];
    if size <= 2 * SAMPLE {
        let mut all = Vec::new();
        file.read_to_end(&mut all)?;
        return Ok(blake3::hash(&all));
    }
    file.read_exact(&mut buffer)?;
    hasher.update(&buffer);
    file.seek(SeekFrom::End(-(SAMPLE as i64)))?;
    file.read_exact(&mut buffer)?;
    hasher.update(&buffer);
    Ok(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::Classifier;
    use crate::index::parse;
    use crate::scan::list_files;

    fn list(dir: &Path) -> Vec<FileEntry> {
        list_files(dir, &Classifier::new(&parse(""))).unwrap()
    }

    fn names(groups: &[Vec<String>]) -> Vec<Vec<String>> {
        groups
            .iter()
            .map(|g| g.iter().map(|p| Path::new(p).file_name().unwrap().to_string_lossy().into_owned()).collect())
            .collect()
    }

    #[test]
    fn by_name_ignores_case_and_folder() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("Logo.png"), "a").unwrap();
        fs::write(dir.path().join("sub/logo.png"), "b").unwrap();
        fs::write(dir.path().join("outro.png"), "a").unwrap();
        let groups = by_name(&list(dir.path()));
        assert_eq!(names(&groups), [["Logo.png", "logo.png"]]);
    }

    #[test]
    fn by_content_finds_copies_with_any_name() {
        let dir = tempfile::tempdir().unwrap();
        let big: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        let mut middle_changed = big.clone();
        middle_changed[150_000] ^= 1; // mesmo tamanho, mesmas pontas, meio diferente
        fs::write(dir.path().join("a.mp4"), &big).unwrap();
        fs::write(dir.path().join("copia.mp4"), &big).unwrap();
        fs::write(dir.path().join("quase.mp4"), &middle_changed).unwrap();
        fs::write(dir.path().join("x.txt"), "igual").unwrap();
        fs::write(dir.path().join("y.txt"), "igual").unwrap();
        fs::write(dir.path().join("z.txt"), "outro").unwrap(); // mesmo tamanho, conteúdo diferente
        fs::write(dir.path().join("vazio1"), "").unwrap();
        fs::write(dir.path().join("vazio2"), "").unwrap();

        let search = Search::default();
        let groups = search.by_content(&list(dir.path()), |_, _| {}).unwrap();
        assert_eq!(names(&groups), [vec!["a.mp4", "copia.mp4"], vec!["x.txt", "y.txt"]]);

        // segunda busca usa o cache e dá o mesmo resultado
        let cached = search.cache.lock().unwrap().len();
        assert!(cached > 0);
        assert_eq!(search.by_content(&list(dir.path()), |_, _| {}).unwrap(), groups);
        assert_eq!(search.cache.lock().unwrap().len(), cached);
    }

    #[test]
    fn unique_sizes_are_never_read() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a"), "1").unwrap();
        fs::write(dir.path().join("b"), "22").unwrap();
        let search = Search::default();
        let reads = std::cell::Cell::new(0);
        assert!(search.by_content(&list(dir.path()), |_, _| reads.set(reads.get() + 1)).unwrap().is_empty());
        assert_eq!(reads.get(), 0);
    }

    #[test]
    fn cancel_stops_the_search() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["a", "b", "c"] {
            fs::write(dir.path().join(name), "mesmo").unwrap();
        }
        let search = Search::default();
        let result = search.by_content(&list(dir.path()), |done, _| {
            if done == 1 {
                search.cancel();
            }
        });
        assert_eq!(result, None);
    }
}

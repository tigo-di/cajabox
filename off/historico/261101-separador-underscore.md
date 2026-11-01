---
periodo: 2026-11-01 a 2026-11-01
commits: 2eb6dc4
tasks: 261101-separador-underscore (concluída)
---

# Underscore como separador

## 1. Separação de termos
- **Solução/decisão:** `_` separa tokens como `-` e espaço, em nomes de arquivo e termos do índice (mesma função `tokenize`). Consequência aceita: `news_241031` agora tem data e `mala_direta` no índice equivale a `mala-direta`.
- **Resultado/testes:** `cargo test` 41 passando; usuário conferiu no programa.

## Pendências
- Repositório remoto (GitHub) e push
- (Opcional) recarregar a configuração quando editada fora do programa

---
name: tasks
description: Gerencia o fluxo de tasks e steps do projeto em off/tasks e off/steps — criar task, gerar steps a partir de uma task, executar um step, concluir task e registrar o histórico em off/historico. Use quando o usuário pedir para criar/listar/executar/concluir tasks ou steps, registrar histórico/resumo, ou invocar /tasks.
---

# Fluxo de tasks e steps

Tudo fica em `off/`. Tasks e steps são ignorados pelo git (locais); só `off/historico/` é versionado.

```
off/tasks/AAMMDD-nome-curto.md            task em andamento
off/steps/AAMMDD-nome-curto/NN-nome.md    steps da task (NN = 01, 02, ...)
off/concluidos/tasks/                     tasks concluídas
off/concluidos/steps/AAMMDD-nome-curto/   steps da task concluída
off/historico/AAMMDD-tema-curto.md        resumo de um período de trabalho (versionado)
```

Nomes de arquivo: minúsculos, sem espaços, sem acentos, palavras separadas por `-`. A pasta de steps tem exatamente o mesmo nome da task (sem `.md`).

## Sem argumento: listar

Mostre as tasks em `off/tasks/` e, para cada uma, os steps com status (`pendente`/`concluido`) e qual é o próximo. Pergunte o que o usuário quer fazer.

## Criar task

Crie `off/tasks/AAMMDD-nome-curto.md` (data de hoje) com:

```markdown
---
status: pendente
criada: AAAA-MM-DD
---

# <Título>

## Objetivo
<o que se quer alcançar e por quê>

## Contexto
<informações relevantes, decisões já tomadas, links>

## Critérios de pronto
- [ ] <resultado verificável>
```

Se o usuário já criou o arquivo (mesmo vazio ou com nome fora do padrão), renomeie para o padrão e complete-o com ele, perguntando o que faltar. Não invente objetivo ou critérios.

## Gerar steps

1. Leia a task e o código envolvido (e o CLAUDE.md).
2. Proponha a lista de steps **no chat** (título + uma linha cada), em ordem de execução. Cada step deve ser pequeno o bastante para ser feito e verificado de uma vez.
3. **Só grave os arquivos após o OK do usuário.** Aceite ajustes na lista antes.
4. Crie `off/steps/<task>/NN-nome.md`:

```markdown
---
status: pendente
---

# NN · <Título>

## Objetivo
<o que este step entrega>

## Arquivos
- <caminho> — <o que muda>

## O que fazer
1. ...

## Como verificar
<comando, teste ou checagem concreta>
```

5. Na task, mude `status` para `em andamento` e crie o branch da task (seção Git).

## Executar step

1. Execute um step por vez, o próximo `pendente` na ordem, salvo se o usuário indicar outro.
2. Antes de alterar código, descreva o que será feito se o step envolver decisão não prevista no arquivo.
3. Rode a verificação do step. Se falhar, relate e não marque como concluído.
4. Ao concluir, mude `status: concluido` no arquivo do step e acrescente ao final:

```markdown
## Resultado
<o que foi feito, desvios em relação ao plano, verificação executada>
```

5. Após o OK do usuário na verificação, faça o commit do step (seção Git).
6. Informe qual é o próximo step.

## Concluir task

Quando todos os steps estiverem `concluido`:
1. Confira os critérios de pronto da task com o usuário e marque os atendidos (`- [x]`).
2. Mude `status: concluido` na task e acrescente `concluida: AAAA-MM-DD`.
3. Mova `off/tasks/<task>.md` → `off/concluidos/tasks/` e `off/steps/<task>/` → `off/concluidos/steps/`.
4. Registre no histórico (seção abaixo).
5. Faça o commit do histórico e o merge em `main` (seção Git).

## Histórico

Registra o panorama de um período de trabalho: problemas, decisões e o **porquê** delas, resultado e pendências. Complementa o `git log` (o que mudou no código) e `off/concluidos/` (como cada task foi executada).

**Quando:** ao concluir uma task, ou quando o usuário pedir um resumo/registro do que foi feito.

**Como:**
1. Se o período em andamento já tem um arquivo em `off/historico/` (mesmo tema, ainda sem pendências fechadas), proponha acrescentar a ele; senão, proponha um arquivo novo `AAMMDD-tema-curto.md` (data de fechamento).
2. Mostre o texto **no chat** e só grave após o OK do usuário.
3. Formato:

```markdown
---
periodo: AAAA-MM-DD a AAAA-MM-DD
commits: <hashes curtos>
tasks: <nome-da-task> (concluída|planejada), ...
---

# <Tema>

## <N>. <Assunto>
- **Problema:** ...
- **Solução/decisão:** ... (e por quê)
- **Resultado/testes:** ...

## Pendências
- ...
```

4. Seja factual e conciso: nada de rascunhos, só o que foi decidido ou feito. Não repita detalhes que já estão no código, nos commits ou nos steps; aponte para eles.
5. `off/historico/` é versionado: o arquivo entra no commit de conclusão da task (ver Git). Não inclua segredos (chaves, senhas, dados pessoais de clientes).

## Git

- **Branch por task:** ao gravar os steps, crie a partir de `main` o branch `task/<nome-da-task>` (ex.: `task/261031-fechar-sobre`) e trabalhe nele até concluir a task.
- **Commit por step:** ao concluir um step (verificação feita e aprovada pelo usuário), faça um commit no branch da task com o código do step. Mensagem: `<Título do step> (step NN)`. Steps e tasks são ignorados pelo git; o commit leva só o código.
- **Conclusão da task:** depois de gravar o histórico, faça o commit do arquivo de `off/historico/` no branch e o merge em `main` com `--no-ff` (mensagem `Task <nome-da-task>`), mantendo o branch.
- **Tag por release:** quando o usuário decidir fechar uma versão, crie em `main` a tag anotada `vX.Y.Z` (semver: X mudança incompatível, Y função nova, Z correção). Sugira a versão; não crie tag sem o OK. Antes da tag, atualize a versão em `package.json` (fonte, lida pelo `tauri.conf.json`) e em `src-tauri/Cargo.toml` (+ `Cargo.lock`) e faça em `main` o commit `Versão X.Y.Z`.
- Antes de cada commit, confira `git status` para não incluir arquivos indevidos (segredos, builds, arquivos pessoais).

## Regras

- Commits, merge e tags seguem a seção Git. Push, deploy e reescrita de histórico (rebase, amend de commit já existente, reset) dependem de pedido explícito do usuário.
- Se, durante a execução, o plano se mostrar errado, pare e proponha a alteração dos steps em vez de desviar silenciosamente.

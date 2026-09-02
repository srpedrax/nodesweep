# Contribuindo com o NodeSweep

Obrigado por querer melhorar o NodeSweep. Como o projeto remove diretórios, mudanças na lógica de limpeza exigem cuidado adicional.

## Ambiente de desenvolvimento

Requisitos: Node.js 18 ou superior, npm, Git, Rust estável e os pré-requisitos Tauri do seu sistema operacional.

```powershell
npm.cmd install
npm.cmd --prefix client install
npm.cmd test
npm.cmd run build
```

Para desenvolver, use `npm.cmd run dev`; o Tauri inicia o Vite e abre a janela desktop.

## Pull requests

1. Crie uma branch a partir de `main`.
2. Mantenha a alteração pequena e focada.
3. Adicione testes para toda mudança no scanner ou na limpeza.
4. Execute os testes e o build antes de enviar.
5. Explique riscos de filesystem e as medidas de segurança adotadas.

Não amplie os alvos removíveis sem discussão prévia. Nesta etapa do 2.0, somente diretórios `node_modules` validados podem ser apagados.

## Estilo

- Rust na camada nativa e módulos ES no cliente.
- Sem dependências novas quando a plataforma já oferece a funcionalidade necessária.
- Mensagens de erro úteis sem expor detalhes internos do servidor.
- Commits claros e no imperativo, preferencialmente seguindo Conventional Commits.

Ao contribuir, você concorda que sua contribuição será licenciada sob a licença MIT do projeto.

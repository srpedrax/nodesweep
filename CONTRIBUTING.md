# Contribuindo com o NodeSweep

Obrigado por querer melhorar o NodeSweep. Como o projeto remove diretórios, mudanças na lógica de limpeza exigem cuidado adicional.

## Ambiente de desenvolvimento

Requisitos: Node.js 18 ou superior, npm e Git.

```powershell
npm.cmd install
npm.cmd --prefix client install
npm.cmd test
npm.cmd run build
```

Para desenvolver, execute o backend com `npm.cmd start` e o frontend com `npm.cmd run dev` em terminais separados.

## Pull requests

1. Crie uma branch a partir de `main`.
2. Mantenha a alteração pequena e focada.
3. Adicione testes para toda mudança no scanner ou na limpeza.
4. Execute os testes e o build antes de enviar.
5. Explique riscos de filesystem e as medidas de segurança adotadas.

Não amplie os alvos removíveis sem discussão prévia. No v0.1, somente diretórios `node_modules` validados podem ser apagados.

## Estilo

- CommonJS no servidor e módulos ES no cliente.
- Sem dependências novas quando a plataforma já oferece a funcionalidade necessária.
- Mensagens de erro úteis sem expor detalhes internos do servidor.
- Commits claros e no imperativo, preferencialmente seguindo Conventional Commits.

Ao contribuir, você concorda que sua contribuição será licenciada sob a licença MIT do projeto.

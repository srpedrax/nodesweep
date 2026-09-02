# Política de segurança

## Versões suportadas

Enquanto o projeto estiver na série inicial, somente a versão mais recente receberá correções de segurança.

| Versão | Suportada |
| --- | --- |
| 0.1.x | Sim |
| anteriores | Não |

## Relatando uma vulnerabilidade

Não abra uma issue pública para vulnerabilidades que possam causar exclusão indevida, travessia de caminhos ou acesso não autorizado a arquivos. Use o recurso **Private vulnerability reporting** do repositório no GitHub.

Inclua, quando possível:

- versão e sistema operacional;
- cenário de reprodução mínimo;
- caminho enviado à API;
- impacto esperado;
- sugestão de mitigação.

Evite executar provas de conceito contra dados reais. Use apenas diretórios temporários descartáveis.

## Modelo de segurança atual

- somente `node_modules` diretamente abaixo de um projeto com `package.json` regular pode ser removido;
- confirmação explícita é obrigatória;
- o lote inteiro é validado antes da primeira remoção;
- symlinks e junctions no caminho são recusados;
- o alvo é revalidado imediatamente antes da exclusão;
- a interface acessa somente comandos nativos registrados; não existe servidor HTTP local;
- diretórios protegidos do sistema operacional são recusados.
- a interface envia IDs de snapshots, nunca caminhos de exclusão;
- categorias Gradle usam uma whitelist explícita e JDKs/configurações não são registradas como alvos.

Nenhuma validação elimina completamente condições de corrida do sistema de arquivos. Faça backup dos projetos importantes e revise a seleção apresentada na confirmação.

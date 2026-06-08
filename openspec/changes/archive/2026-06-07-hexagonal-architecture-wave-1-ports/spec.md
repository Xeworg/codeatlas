# Spec — hexagonal-architecture-wave-1-ports

## Dominios de spec

| Dominio                    | Tipo  | Path                                       |
| -------------------------- | ----- | ------------------------------------------ |
| backend-ports-and-services | nuevo | `specs/backend-ports-and-services/spec.md` |
| error-contract             | nuevo | `specs/error-contract/spec.md`             |
| frontend-service-layer     | nuevo | `specs/frontend-service-layer/spec.md`     |
| ai-module-boundary         | nuevo | `specs/ai-module-boundary/spec.md`         |

## Relación con specs canónicas

Este cambio es **estructural**. No redefine comportamiento funcional de usuario final ya cubierto por `openspec/specs/project-understanding/spec.md`; reorganiza responsabilidades para que ese comportamiento se mantenga con mejores bordes arquitectónicos.

## Notas

- No hay features nuevas en esta ola.
- Los comandos existentes de workspaces/snapshots/comments/health/C4 se consideran parte del refactor porque ya existen en el repo.
- El contrato de error cambia de string opaco a JSON serializado como string en IPC, con fallback frontend durante transición.
- `queries.rs` no se parte internamente en esta ola, pero sí puede recibir adaptación aditiva para implementar puertos.

# CodeAtlas — Plan de Releases v1 → v3

**Versión:** pre-SDD
**Alcance:** cronograma de releases, fases (alpha/beta/GA), criterios de salida

---

## 1. Estrategia de versionado

```
v1.0.0-alpha.1  →  v1.0.0-alpha.2  →  v1.0.0-beta.1  →  v1.0.0-rc.1  →  v1.0.0
                                       ↑
                                  primeros testers externos
```

| Fase | Audiencia | Objetivo | Duración típica |
|---|---|---|---|
| **Alpha** | Dev team + 2-3 early adopters | Validar funcionalidad core, encontrar bugs críticos. | 2 semanas |
| **Beta** | 10-20 testers externos | Validar UX, performance, compatibilidad multi-OS. | 3-4 semanas |
| **RC** | Mismos que beta + widen | Regresión final, no nuevos features. | 1-2 semanas |
| **GA** | Público general | Release estable. | — |

---

## 2. Timeline v1 (MVP)

| Semana | Milestone | Artefacto | Hitos |
|---|---|---|---|
| 1-2 | **Sprint 0** | Setup monorepo, CI, contratos | `tauri dev` funciona. `cargo test` verde. |
| 3-4 | **Sprint 1** | Scanner + parser funcional | Escaneo de fixture 100 archivos < 1s. |
| 5-6 | **Sprint 2** | Grafo file-level | Fixture de 3 archivos → grafo correcto. |
| 7-9 | **Sprint 3** | Diagrama interactivo | Grafo navegable con fixture real. |
| 10-12 | **Sprint 4** | IA contextual | Explain + chat funcional con Anthropic. |
| 13 | **Alpha 1** | Build para Linux (dev) | Demo end-to-end con proyecto real. |
| 14 | **Alpha 2** | Build multiplataforma | Linux + macOS + Windows build funcional. |
| 15-17 | **Beta** | Testers externos | Feedback loop. Bugs P0 resueltos. |
| 18 | **RC1** | Candidato release | 0 bugs P0/P1. Performance dentro de SLAs. |
| 19 | **GA v1.0.0** | Release público | Publicado en GitHub + sitio web. |

---

## 3. Criterios de salida por fase (v1)

### 3.1 Alpha 1
- [ ] Build instalable en Linux (.deb o .AppImage).
- [ ] Flujo completo: abrir proyecto → escanear → ver grafo → explicar nodo.
- [ ] 0 crashes en sesión de 15 min con proyecto de 1000 archivos.
- [ ] Memoria < 500 MB en proyecto mediano.

### 3.2 Alpha 2
- [ ] Build en macOS (.dmg) y Windows (.msi).
- [ ] Mismos criterios que Alpha 1 en las 3 plataformas.
- [ ] Settings UI funcional (configurar API key, tema).

### 3.3 Beta
- [ ] 10 testers externos onboarded.
- [ ] 0 bugs P0 (crash, data loss, security).
- [ ] < 5 bugs P1 (feature core rota).
- [ ] Escaneo < 10s en proyecto de 5000 archivos (medido en 3 máquinas distintas).
- [ ] IA responde en < 5s en el 90% de requests.

### 3.4 RC
- [ ] 0 bugs P0 y P1.
- [ ] Regresión test suite completa (manual checklist + unit tests).
- [ ] Documentación de usuario lista (README, guía de inicio).
- [ ] Performance budget cumplido en las 3 plataformas.

### 3.5 GA
- [ ] Builds firmadas (code signing en macOS/Windows).
- [ ] Página de release en GitHub con changelog.
- [ ] Política de privacidad publicada.
- [ ] Canal de feedback habilitado (GitHub Issues o Discord).

---

## 4. Timeline v2

| Semana | Milestone |
|---|---|
| 1-4 | Insights arquitectónicos (detección + ciclos + hotspots) |
| 5-8 | Impacto de cambios + export |
| 9-10 | Alpha v2 |
| 11-13 | Beta v2 |
| 14 | RC v2 |
| 15 | **GA v2.0.0** |

---

## 5. Timeline v3

| Semana | Milestone |
|---|---|
| 1-4 | Workspace multi-proyecto |
| 5-8 | Snapshots + colaboración local |
| 9-11 | Health score + dashboard ejecutivo |
| 12-14 | Alpha v3 |
| 15-17 | Beta v3 |
| 18 | RC v3 |
| 19 | **GA v3.0.0** |

---

## 6. Canales de distribución

| Canal | v1 | v2 | v3 |
|---|---|---|---|
| **GitHub Releases** | ✅ | ✅ | ✅ |
| **Sitio web** (codeatlas.dev) | ✅ | ✅ | ✅ |
| **Homebrew** (macOS) | Beta | ✅ | ✅ |
| **winget** (Windows) | — | Beta | ✅ |
| **Snap/Flatpak** (Linux) | Beta | ✅ | ✅ |

---

## 7. Estrategia de comunicación

| Hito | Comunicación |
|---|---|
| Alpha 1 interno | Demo grabada para el equipo. |
| Beta pública | Hilo en Twitter/X + Reddit (r/programming, r/typescript). |
| GA v1 | Blog post + Product Hunt launch. |
| v2 | Changelog + email a beta testers. |
| v3 | Case study + conferencias. |

---

## 8. Rollback plan

Si una release GA tiene un bug crítico:
1. **Hotfix:** patch version (v1.0.1) en < 24h si el fix es seguro.
2. **Rollback:** si no hay fix rápido, publicar un aviso y mantener la versión anterior como download alternativo en GitHub Releases.
3. **Root cause:** post-mortem interno en < 48h.

---

## 9. Supuestos

| # | Supuesto | Riesgo si falla |
|---|---|---|
| RL1 | Tauri v2 es estable para producción en Q3 2026 | Retraso de release. |
| RL2 | Anthropic MiniMax sigue disponible y con pricing estable | Cambio de modelo requiere ajustes de prompt. |
| RL3 | 10 testers beta alcanzan para validar UX | Bugs no descubiertos en GA. Mitigación: beta abierta si hace falta. |
| RL4 | Code signing disponible para macOS/Windows en el momento de GA | Sin code signing, instaladores muestran warning de seguridad. |

---

*Documento pre-SDD. Fechas exactas se definen al iniciar cada sprint.*

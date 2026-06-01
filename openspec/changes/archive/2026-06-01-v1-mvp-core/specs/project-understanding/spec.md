# Project Understanding MVP Specification

## Purpose

Definir el comportamiento aceptado de CodeAtlas v1 para comprensión rápida de proyectos mediante escaneo estático, grafo de dependencias file-level y explicación contextual con IA, manteniendo límites estrictos de alcance.

## Requirements

### Requirement: Static Project Scan

The system MUST escanear proyectos de forma estática y segura sin ejecutar código, scripts ni artefactos del proyecto analizado.

#### Scenario: Eligible files are indexed

- GIVEN una carpeta de proyecto válida
- WHEN el usuario inicia `scan_project`
- THEN el sistema MUST indexar archivos `.ts`, `.tsx`, `.js`, `.jsx`, `.rs` y `.json`
- AND el sistema MUST excluir `.git`, `node_modules`, `dist`, `build`, `.next` y `coverage`

#### Scenario: Untrusted input is handled safely

- GIVEN un proyecto con scripts o binarios ejecutables
- WHEN se procesa el escaneo
- THEN el sistema MUST tratar la entrada como no confiable
- AND el sistema MUST NOT ejecutar código del proyecto

### Requirement: Multi-language Parsing for MVP

The system MUST parsear TypeScript, JavaScript y Rust en v1 para extraer metadatos estructurales mínimos del grafo.

#### Scenario: Parser extracts supported symbols

- GIVEN archivos compatibles TS/JS/Rust
- WHEN finaliza el parseo
- THEN el sistema MUST extraer imports y exports
- AND el sistema SHOULD extraer funciones, clases (TS/JS), structs/impl (Rust), interfaces (TS) y enums cuando existan

### Requirement: File-level Dependency Graph

The system MUST construir un grafo dirigido a nivel archivo donde nodos representan archivos y aristas representan imports.

#### Scenario: Graph is produced from imports

- GIVEN un conjunto de archivos con imports resolubles
- WHEN se construye el grafo
- THEN el sistema MUST crear nodos por archivo
- AND el sistema MUST crear aristas dirigidas por relación de import
- AND el sistema MUST NOT incluir nodos intra-archivo (clase/función) en v1

### Requirement: Interactive Graph Exploration

The system MUST proveer exploración visual interactiva del grafo en el layout v1.

#### Scenario: User navigates and inspects graph

- GIVEN un grafo generado
- WHEN el usuario interactúa en la vista central
- THEN el sistema MUST soportar zoom, pan, búsqueda, auto-layout y selección de nodo
- AND el sistema SHOULD resaltar dependencias del nodo seleccionado o enfocado

### Requirement: Explorer and Node Details Synchronization

The system MUST sincronizar el explorer read-only con la selección del grafo y mostrar detalles del nodo activo.

#### Scenario: Explorer selection focuses graph node

- GIVEN un archivo visible en el explorer
- WHEN el usuario selecciona ese archivo
- THEN el sistema MUST enfocar/seleccionar el nodo equivalente en el grafo
- AND el panel de detalles MUST mostrar path, símbolos, dependencias, dependientes y tipo de nodo

### Requirement: Contextual AI Assistant

The system MUST ofrecer explicación por nodo y chat contextual usando contexto acotado, con proveedor primario Anthropic y modelo inicial MiniMax en v1.

#### Scenario: Explain node uses bounded context

- GIVEN un nodo seleccionado
- WHEN el usuario solicita explicación
- THEN el sistema MUST enviar como contexto solo el archivo objetivo, top-5 dependencias y top-3 dependientes
- AND el sistema MUST limitar el contenido del archivo a ~8KB
- AND el sistema MUST NOT enviar el proyecto completo al proveedor IA

#### Scenario: Chat context remains project-grounded

- GIVEN una conversación activa del proyecto
- WHEN el usuario consulta relaciones o responsabilidades
- THEN la respuesta MUST basarse en contexto del proyecto escaneado
- AND el historial de chat MUST mantenerse en memoria en v1 (sin persistencia)

### Requirement: MVP Data Persistence Boundary

The system MUST persistir únicamente el conjunto mínimo de datos definido para v1.

#### Scenario: Minimal schema is enforced

- GIVEN una ejecución de escaneo y construcción de grafo
- WHEN se persisten resultados
- THEN el sistema MUST usar tablas `projects`, `files`, `symbols`, `imports`, `graph_cache` y `ai_config`
- AND el sistema MUST NOT requerir `chat_history` ni `user_settings` avanzadas en v1

### Requirement: Performance and Responsiveness Targets

The system MUST cumplir objetivos de performance del MVP en condiciones objetivo.

#### Scenario: Target project performance budget

- GIVEN un proyecto objetivo de hasta 5000 archivos
- WHEN se ejecuta el flujo abrir→escanear→visualizar
- THEN el escaneo inicial MUST completar en menos de 10 segundos
- AND el primer diagrama SHOULD ser visible en menos de 30 segundos
- AND la interacción de grafo SHOULD mantener latencia percibida menor a 100ms
- AND respuestas IA SHOULD llegar en menos de 5 segundos bajo condiciones nominales

### Requirement: Explicit Out-of-Scope Enforcement

The system MUST mantener bloqueados los no-goals definidos para v1.

#### Scenario: Out-of-scope feature request appears during v1

- GIVEN una solicitud de feature v2/v3 (por ejemplo detección automática de patrones, export Mermaid/PNG/SVG, colaboración, multi-proyecto)
- WHEN se evalúa la implementación en v1
- THEN la solicitud MUST marcarse como fuera de alcance
- AND la implementación MUST diferirse a la versión planificada

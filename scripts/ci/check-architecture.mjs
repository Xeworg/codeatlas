#!/usr/bin/env node
// scripts/ci/check-architecture.mjs
//
// Architecture guard for the codeatlas hexagonal boundary.
// STRICT POLICY: This script does NOT honor any comment-based opt-out
// (e.g. `// arch-allow: ...`). If a forbidden import is reintroduced,
// the build fails. Exceptions require a separate PR with the
// `arch-exception` label and a follow-up spec change.
//
// Usage:
//   node scripts/ci/check-architecture.mjs                  # default scan
//   node scripts/ci/check-architecture.mjs --self-test      # run self-test against fixtures
//
// Exit codes:
//   0 — no violations found
//   1 — at least one forbidden pattern detected (or self-test failed)

import { readFileSync, existsSync, readdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, "..", "..");

// Files scanned in default mode (Rust).
const SCAN_TARGETS = [
  resolve(REPO_ROOT, "src-tauri/src/commands.rs"),
];

// Recursively collect all TypeScript files under src/.
function collectTSTargets(dir) {
  const results = [];
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = resolve(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...collectTSTargets(fullPath));
    } else if (
      entry.name.endsWith(".ts") ||
      entry.name.endsWith(".tsx")
    ) {
      results.push(fullPath);
    }
  }
  return results;
}

const SCAN_TARGETS_TS = collectTSTargets(resolve(REPO_ROOT, "src"));

// Forbidden Rust import patterns. Each entry: { id, regex, description }.
// `regex` is matched against each line of the scanned file. Multi-line
// imports are not supported (Rust single-line `use` statements only).
const FORBIDDEN_PATTERNS = [
  {
    id: "ENGINE_DB_CONCRETE",
    regex: /^\s*use\s+engine::db::/,
    description:
      "Do not import concrete types from engine::db into the presentation layer. Use the port traits in engine::ports (e.g. ScanRepository, GraphRepository, WorkspaceRepository, AnalysisRepository).",
  },
  {
    id: "ENGINE_AI_ANTHROPIC",
    regex: /^\s*use\s+engine::ai::anthropic\b/,
    description:
      "Do not import the private AnthropicProvider adapter. The AI provider must be reached via the public AIService / AIServicePort surface.",
  },
  {
    id: "ENGINE_AI_RESOLVED",
    regex: /^\s*use\s+engine::ai::resolved\b/,
    description:
      "Do not import the private ResolvedProvider enum. Provider resolution is internal to engine::ai::factory.",
  },
  {
    id: "ENGINE_AI_PROVIDER",
    regex: /^\s*use\s+engine::ai::provider::AIProvider\b/,
    description:
      "Do not import the private AIProvider trait. Use AIServicePort (added in PR-B of pre-wave-2-foundation).",
  },
  {
    id: "ENGINE_AI_AISERVICE_CONCRETE",
    regex: /^\s*use\s+engine::ai::AIService\b/,
    description:
      "Do not import the concrete AIService struct. Use Arc<dyn AIServicePort> (PR-B) or the existing engine::ai module re-exports.",
  },
  {
    id: "FRONTEND_SERVICES_IMPORT",
    regex: /from\s+['"](@\/|\.\.\/)services/,
    description:
      "Frontend imports from src/services/ are forbidden — use @/lib/tauri-api directly.",
  },
];

function scanFile(filePath) {
  const violations = [];
  if (!existsSync(filePath)) {
    return violations;
  }
  const content = readFileSync(filePath, "utf8");
  const lines = content.split("\n");
  lines.forEach((line, index) => {
    for (const pattern of FORBIDDEN_PATTERNS) {
      if (pattern.regex.test(line)) {
        violations.push({
          file: filePath,
          line: index + 1,
          patternId: pattern.id,
          snippet: line.trim(),
          description: pattern.description,
        });
      }
    }
  });
  return violations;
}

function runDefaultScan() {
  const allViolations = [];
  for (const target of SCAN_TARGETS) {
    allViolations.push(...scanFile(target));
  }
  for (const target of SCAN_TARGETS_TS) {
    allViolations.push(...scanFile(target));
  }
  if (allViolations.length === 0) {
    console.log("Architecture guard: no violations found.");
    process.exit(0);
  }
  console.error("Architecture guard: forbidden imports detected.");
  console.error("");
  for (const v of allViolations) {
    const relPath = v.file.replace(REPO_ROOT + "/", "");
    console.error(`  ${relPath}:${v.line}  [${v.patternId}]`);
    console.error(`    > ${v.snippet}`);
    console.error(`    ${v.description}`);
    console.error("");
  }
  console.error(`Total: ${allViolations.length} violation(s).`);
  process.exit(1);
}

function runSelfTest() {
  const forbiddenFixtureRS = resolve(
    __dirname,
    "__fixtures__/forbidden.rs",
  );
  const cleanFixtureRS = resolve(__dirname, "__fixtures__/clean.rs");
  const forbiddenFixtureTS = resolve(
    __dirname,
    "__fixtures__/forbidden.ts",
  );
  const cleanFixtureTS = resolve(__dirname, "__fixtures__/clean.ts");

  const forbiddenHitsRS = scanFile(forbiddenFixtureRS);
  const cleanHitsRS = scanFile(cleanFixtureRS);
  const forbiddenHitsTS = scanFile(forbiddenFixtureTS);
  const cleanHitsTS = scanFile(cleanFixtureTS);

  let failed = false;

  // Rust fixtures
  if (forbiddenHitsRS.length === 0) {
    console.error(
      "Self-test FAILED [Rust]: expected forbidden fixture to produce violations, got 0.",
    );
    failed = true;
  } else {
    console.log(
      `Self-test OK [Rust]: forbidden fixture produced ${forbiddenHitsRS.length} violations.`,
    );
  }
  if (cleanHitsRS.length !== 0) {
    console.error(
      `Self-test FAILED [Rust]: expected clean fixture to produce 0 violations, got ${cleanHitsRS.length}.`,
    );
    failed = true;
  } else {
    console.log("Self-test OK [Rust]: clean fixture produced 0 violations.");
  }

  // TypeScript fixtures
  if (forbiddenHitsTS.length === 0) {
    console.error(
      "Self-test FAILED [TS]: expected forbidden.ts fixture to produce violations, got 0.",
    );
    failed = true;
  } else {
    console.log(
      `Self-test OK [TS]: forbidden fixture produced ${forbiddenHitsTS.length} violations.`,
    );
  }
  if (cleanHitsTS.length !== 0) {
    console.error(
      `Self-test FAILED [TS]: expected clean.ts fixture to produce 0 violations, got ${cleanHitsTS.length}.`,
    );
    failed = true;
  } else {
    console.log("Self-test OK [TS]: clean fixture produced 0 violations.");
  }

  process.exit(failed ? 1 : 0);
}

const args = process.argv.slice(2);
if (args.includes("--self-test")) {
  runSelfTest();
} else {
  runDefaultScan();
}
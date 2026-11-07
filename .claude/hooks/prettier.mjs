// PostToolUse hook: formats a .svelte/.ts/.css file under src/ with the project's Prettier after Claude edits it.
// Never fails the tool call: a Prettier error is only reported on stderr.
import { spawnSync } from "node:child_process";
import { relative, resolve } from "node:path";

let input = "";
for await (const chunk of process.stdin) input += chunk;
let payload = {};
try {
  payload = JSON.parse(input || "{}");
} catch (e) {
  console.error(`Prettier hook: unreadable input (${e.message})`);
  process.exit(0);
}
const file = payload.tool_response?.filePath ?? payload.tool_input?.file_path;
const root = process.env.CLAUDE_PROJECT_DIR ?? process.cwd();
if (!file) process.exit(0);

const rel = relative(root, resolve(file)).replaceAll("\\", "/");
if (!/^src\/.+\.(svelte|ts|css)$/.test(rel)) process.exit(0);

const prettier = resolve(root, "node_modules/prettier/bin/prettier.cjs");
const result = spawnSync(process.execPath, [prettier, "--write", rel], { cwd: root, encoding: "utf8" });
if (result.status !== 0) console.error(`Prettier failed on ${rel}: ${result.stderr || result.stdout || result.error}`);

#!/usr/bin/env node
import { spawn } from "node:child_process";
import { createReadStream } from "node:fs";
import { mkdir, stat } from "node:fs/promises";
import { createServer } from "node:http";
import { extname, join, normalize, resolve, sep } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

let baseUrl = (process.env.E2E_BASE_URL || "").replace(/\/$/, "");
const distDir = resolve(process.env.E2E_DIST_DIR || "/private/tmp/payflow-e2e-dist");
let staticServer;

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

async function reachable(url) {
  try {
    const response = await fetch(url, { cache: "no-store" });
    return response.ok;
  } catch {
    return false;
  }
}

async function waitFor(url, timeoutMs = 60_000) {
  const deadline = Date.now() + timeoutMs;
  let lastError = "not requested yet";

  while (Date.now() < deadline) {
    try {
      const response = await fetch(url, { cache: "no-store" });
      if (response.ok) {
        return response;
      }
      lastError = `HTTP ${response.status}`;
    } catch (error) {
      lastError = error.message;
    }
    await delay(250);
  }

  throw new Error(`Timed out waiting for ${url}: ${lastError}`);
}

function assetUrl(path) {
  return new URL(path, `${baseUrl}/`).toString();
}

async function fetchText(path) {
  const response = await waitFor(assetUrl(path));
  const text = await response.text();
  assert(text.length > 0, `${path} should not be empty`);
  return text;
}

async function fetchBytes(path) {
  const response = await waitFor(assetUrl(path));
  const bytes = await response.arrayBuffer();
  assert(bytes.byteLength > 0, `${path} should not be empty`);
  return response;
}

async function run(command, args, options = {}) {
  await new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(command, args, {
      stdio: ["ignore", "pipe", "pipe"],
      ...options,
    });

    child.stdout.on("data", (chunk) => process.stdout.write(chunk));
    child.stderr.on("data", (chunk) => process.stderr.write(chunk));
    child.on("error", rejectPromise);
    child.on("exit", (code) => {
      if (code === 0) {
        resolvePromise();
      } else {
        rejectPromise(new Error(`${command} ${args.join(" ")} exited with ${code}`));
      }
    });
  });
}

async function buildDist() {
  await mkdir(distDir, { recursive: true });
  const env = {
    ...process.env,
    CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR || "/private/tmp/payflow-e2e-target",
  };
  delete env.NO_COLOR;

  await run(
    "trunk",
    ["build", "--dist", distDir, "--color", "never", "--skip-version-check"],
    { env },
  );
}

function contentType(path) {
  switch (extname(path)) {
    case ".css":
      return "text/css";
    case ".html":
      return "text/html";
    case ".js":
      return "text/javascript";
    case ".wasm":
      return "application/wasm";
    default:
      return "application/octet-stream";
  }
}

async function startStaticServer() {
  staticServer = createServer(async (request, response) => {
    try {
      const url = new URL(request.url || "/", "http://127.0.0.1");
      const requestedPath = decodeURIComponent(url.pathname === "/" ? "/index.html" : url.pathname);
      const filePath = normalize(join(distDir, requestedPath));

      if (!filePath.startsWith(`${distDir}${sep}`)) {
        response.writeHead(403);
        response.end("Forbidden");
        return;
      }

      const fileStat = await stat(filePath);
      if (!fileStat.isFile()) {
        response.writeHead(404);
        response.end("Not found");
        return;
      }

      response.writeHead(200, { "content-type": contentType(filePath) });
      createReadStream(filePath).pipe(response);
    } catch {
      response.writeHead(404);
      response.end("Not found");
    }
  });

  await new Promise((resolvePromise, rejectPromise) => {
    staticServer.once("error", rejectPromise);
    staticServer.listen(0, "127.0.0.1", resolvePromise);
  });

  const address = staticServer.address();
  baseUrl = `http://127.0.0.1:${address.port}`;
}

function stopStaticServer() {
  if (staticServer) {
    staticServer.close();
  }
}

async function prepareApp() {
  if (baseUrl && (await reachable(`${baseUrl}/`))) {
    return;
  }

  await buildDist();
  await startStaticServer();
}

function extractAssetPaths(html) {
  const paths = [
    ...html.matchAll(/\b(?:src|href)="([^"#]+)"/g),
    ...html.matchAll(/["']([^"']+\.(?:css|js|wasm))["']/g),
  ]
    .map((match) => match[1])
    .filter((path) => !path.startsWith("http"))
    .map((path) => path.split("?")[0]);

  return [...new Set(paths)];
}

try {
  await prepareApp();

  const html = await fetchText("/");
  assert(html.includes("<title>Payflow Forecast</title>"), "served HTML should include the app title");
  assert(html.includes("styles.css"), "served HTML should include the stylesheet");

  const assetPaths = extractAssetPaths(html);
  assert(assetPaths.some((path) => path.endsWith(".js")), "served HTML should include a generated JS bundle");
  assert(assetPaths.some((path) => path.endsWith("styles.css")), "served HTML should include styles.css");

  for (const path of assetPaths) {
    await fetchBytes(path);
  }

  const jsPath = assetPaths.find((path) => path.endsWith(".js"));
  const js = await fetchText(jsPath);
  const wasmPaths = assetPaths.filter((path) => path.endsWith(".wasm"));
  const fallbackWasmPaths = [...js.matchAll(/[\w.-]+\.wasm/g)].map((match) => match[0]);
  const wasmBundles = wasmPaths.length > 0 ? wasmPaths : [...new Set(fallbackWasmPaths)];
  assert(wasmBundles.length > 0, "generated app should reference a WASM bundle");

  for (const path of wasmBundles) {
    const response = await fetchBytes(path);
    const type = response.headers.get("content-type") || "";
    assert(type.includes("wasm") || type.includes("octet-stream"), `${path} should be served as binary WASM`);
  }

  const css = await fetchText("/styles.css");
  assert(css.includes(".app-shell"), "stylesheet should include the app shell layout");

  console.log(`E2E smoke passed for ${baseUrl}`);
} finally {
  stopStaticServer();
}

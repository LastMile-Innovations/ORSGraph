#!/usr/bin/env node
import { createServer } from "node:http";
import { once } from "node:events";
import { spawn } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const PROTOCOL_VERSION = "2025-11-25";
const DEFAULT_TIMEOUT_MS = 30_000;

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cleanupTasks = [];

async function setupSmokeTarget(options) {
  if (options.mcpUrl) {
    return {
      mcpUrl: normalizeMcpUrl(options.mcpUrl),
      apiLabel: "existing MCP server configuration",
    };
  }

  const api = options.apiBaseUrl
    ? { baseUrl: options.apiBaseUrl, label: options.apiBaseUrl }
    : await startStubApi();
  const mcpPort = options.mcpPort || (await freePort());
  const bind = `127.0.0.1:${mcpPort}`;
  const mcpUrl = `http://${bind}${options.mcpPath}`;
  await startMcpServer({
    bind,
    apiBaseUrl: api.baseUrl,
    bearerToken: options.bearerToken,
    mcpBin: options.mcpBin,
    timeoutMs: options.timeoutMs,
    verbose: options.verbose,
  });
  return { mcpUrl, apiLabel: api.label };
}

async function startStubApi() {
  const server = createServer((request, response) => {
    const url = new URL(request.url || "/", "http://127.0.0.1");
    if (request.method === "GET" && url.pathname === "/api/v1/health") {
      sendJson(response, 200, {
        ok: true,
        service: "orsgraph-api-stub",
        neo4j: "stubbed",
        version: "mcp-smoke",
      });
      return;
    }

    sendJson(response, 404, {
      ok: false,
      error: {
        code: "stub_not_found",
        message: `${request.method} ${url.pathname} is not implemented by the MCP smoke stub`,
      },
    });
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  cleanupTasks.push(
    () =>
      new Promise((resolveClose) => {
        server.close(() => resolveClose());
      }),
  );
  const { port } = server.address();
  return {
    baseUrl: `http://127.0.0.1:${port}/api/v1`,
    label: `stub API http://127.0.0.1:${port}/api/v1`,
  };
}

async function startMcpServer({ bind, apiBaseUrl, bearerToken, mcpBin, timeoutMs, verbose }) {
  const env = {
    ...process.env,
    ORSGRAPH_API_BASE_URL: apiBaseUrl,
    ORSGRAPH_MCP_RATE_LIMIT_REQUESTS:
      process.env.ORSGRAPH_MCP_RATE_LIMIT_REQUESTS || "120",
  };
  if (bearerToken) {
    env.ORSGRAPH_MCP_BEARER_TOKEN = bearerToken;
  }

  const command = mcpBin ? resolve(repoRoot, mcpBin) : "cargo";
  const args = mcpBin
    ? ["--http", "--bind", bind]
    : ["run", "-p", "orsgraph-mcp", "--", "--http", "--bind", bind];
  const child = spawn(command, args, {
    cwd: repoRoot,
    env,
    detached: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const logs = [];
  captureChildOutput(child.stdout, logs, verbose);
  captureChildOutput(child.stderr, logs, verbose);
  cleanupTasks.push(() => terminateProcessGroup(child));

  await waitFor(async () => {
    if (child.exitCode !== null) {
      throw new Error(
        `orsgraph-mcp exited before /healthz became ready.\n${logs.join("").slice(-4000)}`,
      );
    }
    const response = await fetch(`http://${bind}/healthz`).catch(() => null);
    return response?.ok === true;
  }, timeoutMs);
}

async function runSmoke(smoke, options) {
  const client = new McpHttpClient({
    mcpUrl: smoke.mcpUrl,
    bearerToken: options.bearerToken,
    origin: options.origin,
    timeoutMs: options.timeoutMs,
  });

  const initialized = await client.rpc("initialize", {
    protocolVersion: PROTOCOL_VERSION,
    capabilities: {},
    clientInfo: {
      name: "orsgraph-mcp-http-smoke",
      version: "0.1.0",
    },
  });
  const protocolVersion =
    initialized.message?.result?.protocolVersion || PROTOCOL_VERSION;

  await client.notification("notifications/initialized", {});

  const listed = await client.rpc("tools/list", {});
  const tools = listed.message?.result?.tools;
  if (!Array.isArray(tools)) {
    throw new Error("tools/list did not return result.tools[]");
  }
  if (!tools.some((tool) => tool?.name === "orsgraph_health")) {
    const names = tools.map((tool) => tool?.name).filter(Boolean).join(", ");
    throw new Error(`tools/list did not include orsgraph_health; got: ${names}`);
  }

  const health = await client.rpc("tools/call", {
    name: "orsgraph_health",
    arguments: {},
  });
  const healthResult = health.message?.result;
  if (!healthResult || healthResult.isError === true) {
    throw new Error(
      `orsgraph_health returned an MCP tool error: ${JSON.stringify(healthResult)}`,
    );
  }

  return {
    protocolVersion,
    sessionId: client.sessionId,
    toolCount: tools.length,
    healthSummary: summarizeHealthResult(healthResult),
  };
}

class McpHttpClient {
  constructor({ mcpUrl, bearerToken, origin, timeoutMs }) {
    this.mcpUrl = normalizeMcpUrl(mcpUrl);
    this.bearerToken = bearerToken;
    this.origin = origin || new URL(this.mcpUrl).origin;
    this.timeoutMs = timeoutMs;
    this.nextId = 1;
    this.sessionId = null;
  }

  async rpc(method, params) {
    const id = this.nextId++;
    const response = await this.post({ jsonrpc: "2.0", id, method, params });
    const message = response.messages.find((candidate) => candidate?.id === id)
      || response.messages.find((candidate) => candidate?.error || candidate?.result);
    if (!message) {
      throw new Error(`No JSON-RPC response found for ${method}`);
    }
    if (message.error) {
      throw new Error(`${method} failed: ${JSON.stringify(message.error)}`);
    }
    return { message, headers: response.headers };
  }

  async notification(method, params) {
    await this.post({ jsonrpc: "2.0", method, params }, { allowEmpty: true });
  }

  async post(payload, { allowEmpty = false } = {}) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.timeoutMs);
    try {
      const headers = {
        accept: "application/json, text/event-stream",
        "content-type": "application/json",
        origin: this.origin,
        "mcp-protocol-version": PROTOCOL_VERSION,
      };
      if (this.sessionId) {
        headers["mcp-session-id"] = this.sessionId;
      }
      if (this.bearerToken) {
        headers.authorization = `Bearer ${this.bearerToken}`;
      }

      const response = await fetch(this.mcpUrl, {
        method: "POST",
        headers,
        body: JSON.stringify(payload),
        signal: controller.signal,
      });
      const sessionId = response.headers.get("mcp-session-id");
      if (sessionId) {
        this.sessionId = sessionId;
      }
      const text = await response.text();
      if (!response.ok) {
        throw new Error(
          `HTTP ${response.status} from MCP endpoint: ${text.slice(0, 1000)}`,
        );
      }
      if (!text.trim()) {
        if (allowEmpty) {
          return { messages: [], headers: response.headers };
        }
        throw new Error(`Empty response from MCP endpoint for ${payload.method}`);
      }
      return {
        messages: parseMcpMessages(text, response.headers.get("content-type") || ""),
        headers: response.headers,
      };
    } finally {
      clearTimeout(timeout);
    }
  }
}

function parseMcpMessages(text, contentType) {
  if (contentType.includes("application/json")) {
    return [JSON.parse(text)];
  }

  const messages = [];
  let data = [];
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trimEnd();
    if (line.startsWith("data:")) {
      data.push(line.slice(5).trimStart());
      continue;
    }
    if (line === "" && data.length > 0) {
      pushSseMessage(messages, data);
      data = [];
    }
  }
  if (data.length > 0) {
    pushSseMessage(messages, data);
  }

  if (messages.length > 0) {
    return messages;
  }

  return [JSON.parse(text)];
}

function pushSseMessage(messages, dataLines) {
  const payload = dataLines.join("\n").trim();
  if (!payload || payload === "[DONE]") {
    return;
  }
  messages.push(JSON.parse(payload));
}

function summarizeHealthResult(result) {
  const structured =
    result.structuredContent ||
    result.structured_content ||
    parseTextContentJson(result.content);
  if (!structured) {
    return "tool returned a non-empty result";
  }
  const body = structured.body || structured;
  const service = body.service || body.server || "orsgraph-api";
  const ok = body.ok ?? structured.ok;
  const status = structured.status ? ` HTTP ${structured.status}` : "";
  return `${service} ok=${ok}${status}`;
}

function parseTextContentJson(content) {
  if (!Array.isArray(content)) {
    return null;
  }
  for (const item of content) {
    if (item?.type === "text" && typeof item.text === "string") {
      try {
        return JSON.parse(item.text);
      } catch {
        return null;
      }
    }
  }
  return null;
}

function parseArgs(args) {
  const parsed = {
    apiBaseUrl: process.env.ORSGRAPH_API_BASE_URL || "",
    bearerToken: process.env.ORSGRAPH_MCP_BEARER_TOKEN || "",
    help: false,
    mcpBin: process.env.ORSGRAPH_MCP_BIN || "",
    mcpPath: "/mcp",
    mcpPort: 0,
    mcpUrl: process.env.ORSGRAPH_MCP_URL || "",
    origin: process.env.ORSGRAPH_MCP_SMOKE_ORIGIN || "",
    timeoutMs: Number(process.env.ORSGRAPH_MCP_SMOKE_TIMEOUT_MS || DEFAULT_TIMEOUT_MS),
    verbose: false,
  };

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    switch (arg) {
      case "--api-base-url":
        parsed.apiBaseUrl = requiredValue(args, ++index, arg);
        break;
      case "--bearer-token":
        parsed.bearerToken = requiredValue(args, ++index, arg);
        break;
      case "--mcp-path":
        parsed.mcpPath = normalizePath(requiredValue(args, ++index, arg));
        break;
      case "--mcp-bin":
        parsed.mcpBin = requiredValue(args, ++index, arg);
        break;
      case "--mcp-port":
        parsed.mcpPort = Number(requiredValue(args, ++index, arg));
        break;
      case "--mcp-url":
        parsed.mcpUrl = requiredValue(args, ++index, arg);
        break;
      case "--origin":
        parsed.origin = requiredValue(args, ++index, arg);
        break;
      case "--timeout-ms":
        parsed.timeoutMs = Number(requiredValue(args, ++index, arg));
        break;
      case "--verbose":
        parsed.verbose = true;
        break;
      case "--help":
      case "-h":
        parsed.help = true;
        break;
      default:
        throw new Error(`unknown argument: ${arg}`);
    }
  }

  if (!Number.isFinite(parsed.timeoutMs) || parsed.timeoutMs <= 0) {
    throw new Error("--timeout-ms must be a positive number");
  }
  if (!Number.isInteger(parsed.mcpPort) || parsed.mcpPort < 0) {
    throw new Error("--mcp-port must be a non-negative integer");
  }
  return parsed;
}

function requiredValue(args, index, name) {
  const value = args[index];
  if (!value || value.startsWith("--")) {
    throw new Error(`${name} requires a value`);
  }
  return value;
}

function normalizeMcpUrl(raw) {
  const url = new URL(raw);
  return url.toString();
}

function normalizePath(path) {
  const trimmed = path.trim();
  if (!trimmed) {
    return "/mcp";
  }
  return trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
}

function sendJson(response, status, body) {
  response.writeHead(status, { "content-type": "application/json" });
  response.end(JSON.stringify(body));
}

async function freePort() {
  const server = createServer();
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const { port } = server.address();
  await new Promise((resolveClose) => server.close(resolveClose));
  return port;
}

async function waitFor(check, timeoutMs) {
  const started = Date.now();
  let lastError = null;
  while (Date.now() - started < timeoutMs) {
    try {
      if (await check()) {
        return;
      }
    } catch (error) {
      lastError = error;
      if (error.message.includes("exited before")) {
        throw error;
      }
    }
    await new Promise((resolveWait) => setTimeout(resolveWait, 250));
  }
  throw lastError || new Error(`timed out after ${timeoutMs}ms`);
}

function captureChildOutput(stream, logs, verbose) {
  stream.on("data", (chunk) => {
    const text = chunk.toString();
    logs.push(text);
    if (logs.join("").length > 16_000) {
      logs.splice(0, logs.length - 20);
    }
    if (verbose) {
      process.stderr.write(text);
    }
  });
}

async function terminateProcessGroup(child) {
  if (child.exitCode !== null || child.signalCode !== null) {
    return;
  }
  try {
    process.kill(-child.pid, "SIGTERM");
  } catch {
    return;
  }
  const exited = await Promise.race([
    once(child, "exit").then(() => true),
    new Promise((resolveTimeout) => setTimeout(() => resolveTimeout(false), 2_000)),
  ]);
  if (!exited) {
    try {
      process.kill(-child.pid, "SIGKILL");
    } catch {
      // Already gone.
    }
  }
}

async function cleanup() {
  for (const task of cleanupTasks.reverse()) {
    try {
      await task();
    } catch (error) {
      console.error(`cleanup warning: ${error.message}`);
    }
  }
}

function printHelp() {
  console.log(`ORSGraph MCP HTTP smoke

Usage:
  node scripts/smoke-mcp-http.mjs
  node scripts/smoke-mcp-http.mjs --api-base-url http://127.0.0.1:8080/api/v1
  node scripts/smoke-mcp-http.mjs --mcp-url https://mcp.example.com/mcp --bearer-token "$TOKEN"

Default behavior:
  - starts a stub ORSGraph API when --api-base-url is omitted
  - starts orsgraph-mcp over Streamable HTTP when --mcp-url is omitted
  - initializes MCP, sends notifications/initialized, lists tools, calls orsgraph_health

Options:
  --api-base-url <url>   Use a real ORSGraph API when starting a local MCP server
  --mcp-url <url>        Target an already-running MCP HTTP endpoint
  --mcp-bin <path>       Start this built MCP binary instead of cargo run
  --bearer-token <token> Send Authorization: Bearer <token>
  --mcp-port <port>      Local MCP port when starting the server; default is random
  --mcp-path <path>      Local MCP path when starting the server; default /mcp
  --origin <origin>      Origin header; default is the MCP URL origin
  --timeout-ms <ms>      Startup/request timeout; default ${DEFAULT_TIMEOUT_MS}
  --verbose              Stream child process logs
`);
}

async function main() {
  let options = { verbose: false };
  try {
    options = parseArgs(process.argv.slice(2));
    if (options.help) {
      printHelp();
      return;
    }

    const smoke = await setupSmokeTarget(options);
    const result = await runSmoke(smoke, options);
    console.log("ORSGraph MCP HTTP smoke passed");
    console.log(`- MCP endpoint: ${smoke.mcpUrl}`);
    console.log(`- API target: ${smoke.apiLabel}`);
    console.log(`- Protocol: ${result.protocolVersion}`);
    console.log(`- Session: ${result.sessionId || "stateless/no-session-header"}`);
    console.log(`- Tools listed: ${result.toolCount}`);
    console.log(`- orsgraph_health: ${result.healthSummary}`);
  } catch (error) {
    console.error(`ORSGraph MCP HTTP smoke failed: ${error.message}`);
    if (options.verbose && error.stack) {
      console.error(error.stack);
    }
    process.exitCode = 1;
  } finally {
    await cleanup();
  }
}

await main();

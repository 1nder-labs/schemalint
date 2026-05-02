import { describe, it, expect } from 'vitest';
import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const fixturesDir = path.join(__dirname, 'fixtures');
const packageRoot = path.join(__dirname, '..', '..');
const binPath = path.join(packageRoot, 'bin', 'schemalint-zod.js');

function sendJsonRpc(
  child: ReturnType<typeof spawn>,
  request: object
): Promise<string> {
  return new Promise((resolve, reject) => {
    const line = JSON.stringify(request);
    const timeout = setTimeout(() => {
      child.kill();
      reject(new Error('Response timeout'));
    }, 15000);

    let buffer = '';
    const onData = (data: Buffer) => {
      buffer += data.toString();
      const newlineIndex = buffer.indexOf('\n');
      if (newlineIndex !== -1) {
        const response = buffer.slice(0, newlineIndex);
        clearTimeout(timeout);
        child.stdout?.removeListener('data', onData);
        resolve(response);
      }
    };

    child.stdout?.on('data', onData);
    child.stdin?.write(line + '\n');
  });
}

function sendRaw(
  child: ReturnType<typeof spawn>,
  raw: string
): Promise<string> {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      child.kill();
      reject(new Error('Response timeout'));
    }, 5000);

    let buffer = '';
    const onData = (data: Buffer) => {
      buffer += data.toString();
      const newlineIndex = buffer.indexOf('\n');
      if (newlineIndex !== -1) {
        const response = buffer.slice(0, newlineIndex);
        clearTimeout(timeout);
        child.stdout?.removeListener('data', onData);
        resolve(response);
      }
    };

    child.stdout?.on('data', onData);
    child.stdin?.write(raw + '\n');
  });
}

function spawnServer() {
  return spawn('npx', ['tsx', binPath], {
    cwd: fixturesDir,
    stdio: ['pipe', 'pipe', 'pipe'],
    env: { ...process.env },
  });
}

async function waitForExit(
  child: ReturnType<typeof spawn>,
  timeoutMs = 3000
): Promise<number | null> {
  // Check if already exited
  if (child.exitCode !== null) return child.exitCode;

  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      child.kill();
      resolve(null);
    }, timeoutMs);
    child.on('exit', (code) => {
      clearTimeout(timer);
      resolve(code);
    });
  });
}

describe('JSON-RPC server', () => {
  it('responds to discover request with valid models', async () => {
    const child = spawnServer();

    try {
      const responseStr = await sendJsonRpc(child, {
        jsonrpc: '2.0',
        method: 'discover',
        params: { source: 'simple.ts' },
        id: 1,
      });

      const response = JSON.parse(responseStr);
      expect(response.jsonrpc).toBe('2.0');
      expect(response.id).toBe(1);
      expect(response.error).toBeUndefined();
      expect(response.result).toBeDefined();
      expect(response.result.models).toHaveLength(1);

      const model = response.result.models[0];
      expect(model.name).toBe('UserSchema');
      expect(model.schema).toHaveProperty('type', 'object');
      expect(model.source_map).toHaveProperty('/properties/email');

      // Send shutdown
      const shutdownStr = await sendJsonRpc(child, {
        jsonrpc: '2.0',
        method: 'shutdown',
        id: 2,
      });
      expect(JSON.parse(shutdownStr).result).toBe('ok');

      // Close stdin so the readline interface unblocks and process exits
      child.stdin?.end();

      const exitCode = await waitForExit(child);
      expect(exitCode).toBe(0);
    } finally {
      child.kill();
    }
  }, 30000);

  it('returns error for unknown method', async () => {
    const child = spawnServer();

    try {
      const responseStr = await sendJsonRpc(child, {
        jsonrpc: '2.0',
        method: 'unknownMethod',
        id: 1,
      });

      const response = JSON.parse(responseStr);
      expect(response.error).toBeDefined();
      expect(response.error.code).toBe(-32601);
    } finally {
      child.kill();
    }
  }, 15000);

  it('returns parse error for malformed JSON', async () => {
    const child = spawnServer();

    try {
      const responseStr = await sendRaw(child, 'not json{{{');
      const response = JSON.parse(responseStr);
      expect(response.error).toBeDefined();
      expect(response.error.code).toBe(-32700);
    } finally {
      child.kill();
    }
  }, 15000);

  it('returns error for missing jsonrpc field', async () => {
    const child = spawnServer();

    try {
      const responseStr = await sendRaw(
        child,
        '{"method":"discover","id":1}'
      );
      const response = JSON.parse(responseStr);
      expect(response.error).toBeDefined();
      expect(response.error.code).toBe(-32600);
    } finally {
      child.kill();
    }
  }, 15000);
});

import { discoverZodSchemas } from './discover.js';
const METHODS = new Set(['discover', 'shutdown']);
function sendResponse(id, result) {
    const response = { jsonrpc: '2.0', result, id };
    process.stdout.write(JSON.stringify(response) + '\n');
}
function sendError(id, code, message, data) {
    const error = {
        code,
        message,
    };
    if (data !== undefined)
        error.data = data;
    const response = { jsonrpc: '2.0', error, id };
    process.stdout.write(JSON.stringify(response) + '\n');
}
function handleDiscover(request, reqId) {
    const params = request.params ?? {};
    const source = params.source;
    if (!source || typeof source !== 'string') {
        return Promise.resolve(sendError(reqId, -32602, "Missing or invalid 'source' parameter"));
    }
    return discoverZodSchemas(source)
        .then((result) => {
        sendResponse(reqId, result);
    })
        .catch((err) => {
        const message = err instanceof Error ? err.message : String(err);
        sendError(reqId, -32603, `Discovery failed: ${message}`);
    });
}
async function processLine(line) {
    const trimmed = line.trim();
    if (!trimmed)
        return true;
    let request;
    try {
        request = JSON.parse(trimmed);
    }
    catch {
        sendError(null, -32700, 'Parse error');
        return true;
    }
    if (request.jsonrpc !== '2.0') {
        sendError(request.id, -32600, 'Invalid JSON-RPC: missing or incorrect jsonrpc field');
        return true;
    }
    const method = request.method ?? '';
    const reqId = request.id;
    if (method === 'discover') {
        await handleDiscover(request, reqId);
        return true;
    }
    if (method === 'shutdown') {
        sendResponse(reqId, 'ok');
        return false;
    }
    if (method === '') {
        sendError(reqId, -32600, 'Invalid JSON-RPC request: missing method');
        return true;
    }
    sendError(reqId, -32601, `Method not found: ${method}`);
    return true;
}
export async function main() {
    const readline = await import('node:readline');
    const rl = readline.createInterface({
        input: process.stdin,
        crlfDelay: Infinity,
    });
    for await (const line of rl) {
        const shouldContinue = await processLine(line);
        if (!shouldContinue)
            break;
    }
}
//# sourceMappingURL=server.js.map
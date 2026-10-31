// Minimal client for the owner's already-configured Penpot MCP connection.
// Never prints the configured URL, which may contain a credential.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';

const [operation, argument, destination, platform = 'Windows'] = process.argv.slice(2);
let endpoint = process.env.PENPOT_MCP_URL;
if (!endpoint) {
  const config = await readFile(join(homedir(), '.codex', 'config.toml'), 'utf8');
  const section = config.split(/(?=^\[)/m).find(s => /^\[mcp_servers\.penpot_local\]/.test(s));
  endpoint = section?.match(/^url\s*=\s*"([^"]+)"/m)?.[1];
}
if (!endpoint) throw new Error('Set PENPOT_MCP_URL or configure mcp_servers.penpot_local.');
let sequence = 0;
let session;
async function rpc(method, params) {
  const id = ++sequence;
  let response;
  try { response = await fetch(endpoint, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
      ...(session ? { 'Mcp-Session-Id': session } : {}),
    },
    body: JSON.stringify({ jsonrpc: '2.0', id, method, params }),
    signal: AbortSignal.timeout(90000),
  }); } catch {
    throw new Error('Cannot reach the configured Penpot server; check network access and MCP connection.');
  }
  session = response.headers.get('mcp-session-id') || session;
  const body = await response.text();
  const reply = response.headers.get('content-type')?.includes('application/json')
    ? JSON.parse(body)
    : body.split('\n').filter(l => l.startsWith('data: '))
      .map(l => JSON.parse(l.slice(6))).find(item => item.id === id);
  if (!response.ok || !reply || reply.error) {
    throw new Error(`Penpot RPC ${method} failed (${response.status}).`);
  }
  return reply.result;
}
await rpc('initialize', {
  protocolVersion: '2024-11-05', capabilities: {},
  clientInfo: { name: 'lightcal-design', version: '1.0' },
});
if (operation === 'tools') {
  const listing = await rpc('tools/list', {});
  console.log(JSON.stringify(listing.tools.map(tool => ({ name: tool.name, description: tool.description })), null, 2));
  process.exit(0);
}
let name;
let args;
if (operation === 'run' || operation === 'report') {
  name = 'execute_code';
  let code = await readFile(argument, 'utf8');
  if (!['Windows', 'macOS'].includes(platform)) throw new Error('Design platform must be Windows or macOS.');
  if (operation === 'run' && destination) {
    const match = destination.match(/^(\d+):(\d+)$/);
    if (!match) throw new Error('Optional run batch must be START:END, end exclusive.');
    code = `storage.refinement.batch = {start:${Number(match[1])},end:${Number(match[2])}};\n` + code;
  } else if (!argument.endsWith('refinement.js')) {
    code = 'if(storage.refinement)storage.refinement.batch=null;\n' + code;
  }
  if (argument.endsWith('refinement-states.js')) code = `storage.refinement.statePlatform = '${platform}';\n` + code;
  args = { code };
} else if (operation === 'eval') {
  name = 'execute_code';
  args = { code: argument };
} else if (operation === 'export') {
  name = 'export_shape';
  args = { shapeId: argument, format: destination?.endsWith('.svg') ? 'svg' : 'png' };
} else if (operation === 'api') {
  name = 'penpot_api_info';
  args = { type: argument, ...(destination ? { member: destination } : {}) };
} else if (operation === 'snapshot') {
  name = 'execute_code';
  args = { code: `const v=await penpot.currentFile.saveVersion(${JSON.stringify(argument || 'Lightcal design snapshot')});if(v.isAutosave)await v.pin();return {label:v.label,createdAt:v.createdAt,isAutosave:v.isAutosave};` };
} else {
  throw new Error('Usage: local-client.mjs run FILE [START:END] [Windows|macOS] | report FILE JSON | eval CODE | export ID FILE | api TYPE [MEMBER] | snapshot LABEL');
}
const result = await rpc('tools/call', { name, arguments: args });
if (result.isError) throw new Error(JSON.stringify(result.content));
const failure = result.content?.find(item => item.type === 'text' && /^(Tool execution failed:|Error:)/.test(item.text));
if (failure) throw new Error(failure.text);
if (operation === 'report') {
  const payload = JSON.parse(result.content.find(item => item.type === 'text').text).result;
  await mkdir(dirname(destination), { recursive: true });
  await writeFile(destination, JSON.stringify(payload, null, 2) + '\n');
  console.log(`Saved report ${destination}`);
} else if (operation === 'export') {
  const image = result.content.find(item => item.type === 'image');
  const svg = result.content.find(item => item.type === 'text');
  if (!image && !svg) throw new Error('Penpot did not return an image.');
  await mkdir(dirname(destination), { recursive: true });
  await writeFile(destination, image ? Buffer.from(image.data, 'base64') : svg.text);
  console.log(`Exported ${destination}`);
} else {
  for (const item of result.content || []) if (item.type === 'text') console.log(item.text);
}

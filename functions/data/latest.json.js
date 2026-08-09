// Serverer /data/latest.json fra KV. Dataene ligger aldri i git eller som
// statisk fil — kun her, bak Cloudflare Access.
export async function onRequest({ env }) {
  const body = await env.DATA.get("latest.json");
  return new Response(body ?? '{"generated_at":"","news":[],"surf":[]}', {
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "no-store",
    },
  });
}

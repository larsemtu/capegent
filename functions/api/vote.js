// Stemme på events (👍/👎) — lagres i KV og mates inn i AI-kurateringen
// ved neste innsamling. Ligger bak Cloudflare Access som resten av siden.
export async function onRequestPost({ request, env }) {
  let body;
  try {
    body = await request.json();
  } catch {
    return new Response('ugyldig json', { status: 400 });
  }
  const { url, title, vote } = body;
  if (!url || !title || !['up', 'down'].includes(vote)) {
    return new Response('mangler felter', { status: 400 });
  }
  const raw = await env.DATA.get('votes.json');
  let votes = [];
  try { votes = JSON.parse(raw) || []; } catch {}
  // Én stemme per event-url — ny stemme overskriver
  votes = votes.filter(v => v.url !== url);
  votes.push({ url, title: String(title).slice(0, 120), vote, ts: new Date().toISOString() });
  if (votes.length > 400) votes = votes.slice(-400);
  await env.DATA.put('votes.json', JSON.stringify(votes));
  return new Response(JSON.stringify({ ok: true, count: votes.length }), {
    headers: { 'content-type': 'application/json' },
  });
}

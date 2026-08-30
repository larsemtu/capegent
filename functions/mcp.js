// MCP-server (streamable HTTP) for dashborddataene. Kobles til fra
// Claude Code / Claude Desktop / claude.ai så Claude kan hente vær, surf,
// events, kalender m.m. direkte. Auth: Bearer-token eller ?key= (MCP_TOKEN
// som Pages-secret). Stien /mcp har Access-bypass — tokenet er låsen.

const TOOLS = [
  {
    name: 'dashboard_overview',
    description:
      'Short summary of the whole Cape Town dashboard: current weather, surf, ' +
      'upcoming calendar entries, open tasks and recommended events.',
    inputSchema: { type: 'object', properties: {} },
    run: d => {
      const w = d.weather;
      const lines = [];
      if (w) lines.push(`WEATHER Green Point: ${Math.round(w.temp_c)}°C, ${w.symbol}, wind ${Math.round(w.wind_ms)} m/s. Precip next hour: ${w.precipitation_mm} mm.`);
      for (const s of d.surf || []) lines.push(`SURF ${s.name}: ${s.rating} — swell ${s.swell_height_m.toFixed(1)}m @ ${s.swell_period_s.toFixed(0)}s, wind ${Math.round(s.wind_ms)} m/s${s.water_temp_c != null ? `, water ${Math.round(s.water_temp_c)}°` : ''}`);
      if (d.surf_summary) lines.push(`SURF READ: ${d.surf_summary}`);
      for (const e of (d.calendar || []).slice(0, 5)) lines.push(`CALENDAR: ${e.start} ${e.title}`);
      for (const t of d.todos || []) lines.push(`TASK [${t.status}]: ${t.title}${t.due ? ` (due ${t.due})` : ''}`);
      for (const e of (d.events || []).filter(e => (e.relevance || 0) >= 4).slice(0, 6)) lines.push(`EVENT [${e.relevance}/5] ${e.start.slice(0, 10)}: ${e.title} @ ${e.venue}`);
      if (d.load_shedding) lines.push(`LOAD SHEDDING: stage ${d.load_shedding.stage}`);
      lines.push(`(data generated ${d.generated_at})`);
      return lines.join('\n');
    },
  },
  {
    name: 'weather',
    description: 'Weather forecast for Green Point, Cape Town (Yr/MET data): now, hourly and the days ahead. Wind in m/s, precip in mm.',
    inputSchema: {
      type: 'object',
      properties: { days: { type: 'number', description: 'Days ahead (1-10, default 5)' } },
    },
    run: (d, args) => {
      const w = d.weather;
      if (!w) return 'No weather data.';
      const days = Math.min(Math.max(args.days || 5, 1), 10);
      const lines = [`Now: ${Math.round(w.temp_c)}°C, ${w.symbol}, wind ${Math.round(w.wind_ms)} m/s from ${Math.round(w.wind_direction_deg)}°`];
      lines.push('\nNext 12 hours:');
      for (const h of (w.hourly || []).slice(0, 12)) {
        lines.push(`  ${h.time.slice(11, 16)}: ${Math.round(h.temp_c)}° ${h.symbol}, ${h.precipitation_mm} mm, ${Math.round(h.wind_ms)} m/s`);
      }
      lines.push('\nDays ahead:');
      for (const day of (w.days || []).slice(0, days)) {
        lines.push(`  ${day.date}: ${Math.round(day.temp_min_c)}-${Math.round(day.temp_max_c)}°C, ${day.precipitation_mm_total.toFixed(1)} mm total, wind up to ${Math.round(day.wind_max_ms)} m/s`);
        for (const p of day.periods) lines.push(`    ${String(p.from_hour).padStart(2, '0')}: ${p.symbol}, ${Math.round(p.temp_c)}°, ${p.precipitation_mm} mm`);
      }
      return lines.join('\n');
    },
  },
  {
    name: 'surf',
    description: 'Surf conditions for Muizenberg, Big Bay and Llandudno: rating (poor→good), swell, wind, tide, water temp + AI read on the best windows next 48h.',
    inputSchema: { type: 'object', properties: {} },
    run: d => {
      const lines = [];
      for (const s of d.surf || []) {
        lines.push(`## ${s.name}: ${s.rating.toUpperCase()}`);
        lines.push(`Swell ${s.swell_height_m.toFixed(1)}m @ ${s.swell_period_s.toFixed(0)}s, wind ${Math.round(s.wind_ms)} m/s${s.water_temp_c != null ? `, water ${s.water_temp_c.toFixed(1)}°C` : ''}`);
        if (s.analysis) lines.push(s.analysis);
        lines.push('Next 24h (every 3 hours):');
        for (const h of (s.hourly || []).filter((_, i) => i % 3 === 0).slice(0, 8)) {
          lines.push(`  ${h.time.slice(11, 16)}: ${h.rating} — ${h.swell_height_m.toFixed(1)}m @ ${h.swell_period_s.toFixed(0)}s, wind ${Math.round(h.wind_ms)} m/s${h.tide_m != null ? `, tide ${h.tide_m >= 0 ? '+' : ''}${h.tide_m.toFixed(1)}m` : ''}`);
        }
        lines.push('');
      }
      if (d.surf_summary) lines.push(`OVERALL: ${d.surf_summary}`);
      return lines.join('\n');
    },
  },
  {
    name: 'events',
    description: 'Upcoming Cape Town events, AI-curated against the interest profile (relevance 1-5). Can filter to recommended only.',
    inputSchema: {
      type: 'object',
      properties: { recommended_only: { type: 'boolean', description: 'Only relevance 4-5 (default true)' } },
    },
    run: (d, args) => {
      const kun = args.recommended_only !== false;
      const evs = (d.events || []).filter(e => !kun || (e.relevance || 0) >= 4);
      return evs.map(e =>
        `[${e.relevance || '?'}/5] ${e.start.slice(0, 10)}: ${e.title} @ ${e.venue}${e.why ? `\n    → ${e.why}` : ''}\n    ${e.url}`
      ).join('\n') || 'No events.';
    },
  },
  {
    name: 'news',
    description: 'Cape Town news, prioritized (crime first). Short and long summaries with source links.',
    inputSchema: {
      type: 'object',
      properties: { count: { type: 'number', description: 'Number of stories (default 8)' } },
    },
    run: (d, args) =>
      (d.news || []).slice(0, args.count || 8).map(n =>
        `[${n.category}/${n.urgency}] ${n.headline}\n${n.summary}\n${n.source_url}`
      ).join('\n\n') || 'No news.',
  },
  {
    name: 'calendar_and_tasks',
    description: 'Upcoming entries from the shared calendar and open Linear tasks (CapeTasks).',
    inputSchema: { type: 'object', properties: {} },
    run: d => {
      const lines = ['## Calendar'];
      for (const e of d.calendar || []) lines.push(`${e.start}${e.end ? ` → ${e.end}` : ''}: ${e.title}`);
      lines.push('', '## Tasks');
      for (const t of d.todos || []) lines.push(`[${t.status}] ${t.title}${t.due ? ` (due ${t.due})` : ''}${t.project ? ` · ${t.project}` : ''}`);
      return lines.join('\n');
    },
  },
];

export async function onRequest({ request, env }) {
  const url = new URL(request.url);
  const auth = (request.headers.get('authorization') || '').replace(/^Bearer\s+/i, '');
  const key = auth || url.searchParams.get('key') || '';
  if (!env.MCP_TOKEN || key !== env.MCP_TOKEN) {
    return new Response('unauthorized', { status: 401 });
  }
  if (request.method !== 'POST') {
    return new Response('capegent MCP — POST JSON-RPC (streamable http)', { status: 405 });
  }

  let rpc;
  try { rpc = await request.json(); } catch { return new Response('bad json', { status: 400 }); }

  const reply = result => new Response(
    JSON.stringify({ jsonrpc: '2.0', id: rpc.id, result }),
    { headers: { 'content-type': 'application/json' } },
  );

  if (rpc.method === 'initialize') {
    return reply({
      protocolVersion: rpc.params?.protocolVersion || '2025-03-26',
      capabilities: { tools: {} },
      serverInfo: { name: 'capegent', version: '1.0.0' },
    });
  }
  if (!('id' in rpc)) return new Response(null, { status: 202 }); // notifications

  if (rpc.method === 'tools/list') {
    return reply({ tools: TOOLS.map(({ name, description, inputSchema }) => ({ name, description, inputSchema })) });
  }
  if (rpc.method === 'tools/call') {
    const tool = TOOLS.find(t => t.name === rpc.params?.name);
    if (!tool) return reply({ content: [{ type: 'text', text: 'unknown tool' }], isError: true });
    let data = {};
    try { data = JSON.parse(await env.DATA.get('latest.json')) || {}; } catch {}
    try {
      return reply({ content: [{ type: 'text', text: tool.run(data, rpc.params?.arguments || {}) }] });
    } catch (e) {
      return reply({ content: [{ type: 'text', text: 'error: ' + e.message }], isError: true });
    }
  }
  return new Response(
    JSON.stringify({ jsonrpc: '2.0', id: rpc.id, error: { code: -32601, message: 'method not found' } }),
    { headers: { 'content-type': 'application/json' } },
  );
}

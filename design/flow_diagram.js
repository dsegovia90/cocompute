/** @schema 2.10 */

const W = pencil.width;
const H = pencil.height;

const COL = ['#22D3EE', '#C084FC', '#34D399'];

const CL = [
  { id: 'c0', label: 'Desktop', sub: 'workstation', icon: 'monitor' },
  { id: 'c1', label: 'Mobile', sub: 'smartphone', icon: 'smartphone' },
  { id: 'c2', label: 'Laptop', sub: 'notebook', icon: 'laptop' },
];
const GP = [
  { id: 'g0', label: 'Host GPU', sub: 'RTX 4090' },
  { id: 'g1', label: 'Host GPU', sub: 'A100 80GB' },
  { id: 'g2', label: 'Host GPU', sub: 'RTX 3090' },
  { id: 'g3', label: 'Host GPU', sub: 'H100 SXM' },
  { id: 'g4', label: 'Host GPU', sub: 'RTX 4080' },
];

const cw = Math.max(Math.min(W * 0.115, 96), 80);
const ch = Math.max(Math.min(H * 0.20, 88), 80);
const gpW = Math.max(Math.min(W * 0.155, 138), 130);
const gpH = Math.max(Math.min(H * 0.112, 50), 48);
const hubW = Math.max(Math.min(W * 0.13, 115), 115);
const hubH = Math.max(Math.min(H * 0.24, 108), 100);
const my = H * 0.5;

const cards = {};
cards.hub = { x: W * 0.5 - hubW / 2, y: my - hubH / 2, w: hubW, h: hubH, cx: W * 0.5, cy: my, rad: 16 };

const clSpan = H * 0.56;
const clSy = my - clSpan / 2;
for (let i = 0; i < CL.length; i++) {
  const yy = clSy + clSpan * i / (CL.length - 1) - ch / 2;
  cards[CL[i].id] = { x: W * 0.06, y: yy, w: cw, h: ch, cx: W * 0.06 + cw / 2, cy: yy + ch / 2, rad: 14 };
}

const gpSpan = H * 0.82;
const gpSy = my - gpSpan / 2;
for (let i = 0; i < GP.length; i++) {
  const yy = gpSy + gpSpan * i / (GP.length - 1) - gpH / 2;
  const xx = W * 0.94 - gpW;
  cards[GP[i].id] = { x: xx, y: yy, w: gpW, h: gpH, cx: xx + gpW / 2, cy: yy + gpH / 2, rad: 12 };
}

const pths = {};
for (let i = 0; i < CL.length; i++) {
  const cd = cards[CL[i].id];
  const hb = cards.hub;
  const sx = cd.x + cd.w;
  const ex = hb.x;
  const mx = sx + (ex - sx) * 0.5;
  pths[CL[i].id + '_hub'] = [sx, cd.cy, mx, cd.cy, mx, hb.cy, ex, hb.cy];
}
for (let i = 0; i < GP.length; i++) {
  const gd = cards[GP[i].id];
  const hb = cards.hub;
  const sx = hb.x + hb.w;
  const ex = gd.x;
  const mx = sx + (ex - sx) * 0.5;
  pths['hub_' + GP[i].id] = [sx, hb.cy, mx, hb.cy, mx, gd.cy, ex, gd.cy];
}

function bz(p, t) {
  const u = 1 - t;
  return {
    x: u * u * u * p[0] + 3 * u * u * t * p[2] + 3 * u * t * t * p[4] + t * t * t * p[6],
    y: u * u * u * p[1] + 3 * u * u * t * p[3] + 3 * u * t * t * p[5] + t * t * t * p[7],
  };
}

function alphaHex(hex, alpha) {
  const a = Math.max(0, Math.min(255, Math.round(alpha * 255)));
  return hex + a.toString(16).padStart(2, '0').toUpperCase();
}

const nodes = [];

for (const k of Object.keys(pths)) {
  const p = pths[k];
  const geom = 'M ' + p[0].toFixed(1) + ' ' + p[1].toFixed(1)
    + ' C ' + p[2].toFixed(1) + ' ' + p[3].toFixed(1)
    + ' ' + p[4].toFixed(1) + ' ' + p[5].toFixed(1)
    + ' ' + p[6].toFixed(1) + ' ' + p[7].toFixed(1);
  nodes.push({
    type: 'path',
    x: 0, y: 0, width: W, height: H,
    viewBox: [0, 0, W, H],
    geometry: geom,
    stroke: { fill: '#8278C81A', thickness: 1, align: 'center' },
  });
}

const inflight = [
  { pathKey: 'c1_hub', t: 0.6, color: COL[0] },
  { pathKey: 'hub_g2', t: 0.65, color: COL[1] },
  { pathKey: 'c0_hub', t: 0.42, color: COL[2] },
  { pathKey: 'hub_g1', t: 0.55, color: COL[0] },
  { pathKey: 'hub_g3', t: 0.48, color: COL[2] },
];

const sz = 2.2;
const trailLen = 0.42;

for (const r of inflight) {
  const p = pths[r.pathKey];
  if (!p) continue;
  const segments = 14;
  const t0 = Math.max(0, r.t - trailLen);
  for (let s = 0; s < segments; s++) {
    const ta = t0 + (r.t - t0) * (s / segments);
    const tb = t0 + (r.t - t0) * ((s + 1) / segments);
    const pa = bz(p, ta);
    const pb = bz(p, tb);
    const a = (s + 0.5) / segments;
    const lineW = sz * (0.25 + a * 0.75);
    const lineAlpha = a * a * 0.55;
    nodes.push({
      type: 'path',
      x: 0, y: 0, width: W, height: H,
      viewBox: [0, 0, W, H],
      geometry: 'M ' + pa.x.toFixed(2) + ' ' + pa.y.toFixed(2) + ' L ' + pb.x.toFixed(2) + ' ' + pb.y.toFixed(2),
      stroke: { fill: alphaHex(r.color, lineAlpha), thickness: lineW, align: 'center' },
    });
  }
  const head = bz(p, r.t);
  nodes.push({
    type: 'ellipse',
    x: head.x - sz * 6, y: head.y - sz * 6,
    width: sz * 12, height: sz * 12,
    fill: alphaHex(r.color, 0.18),
    effect: { type: 'shadow', shadowType: 'outer', blur: 22, spread: 1, color: r.color, offset: { x: 0, y: 0 } },
  });
  nodes.push({
    type: 'ellipse',
    x: head.x - sz * 0.75, y: head.y - sz * 0.75,
    width: sz * 1.5, height: sz * 1.5,
    fill: '#FFFFFFE0',
  });
}

const clientIcons = { c0: 'monitor', c1: 'smartphone', c2: 'laptop' };

for (let i = 0; i < CL.length; i++) {
  const c = CL[i];
  const cd = cards[c.id];
  nodes.push({
    type: 'frame',
    x: cd.x, y: cd.y, width: cd.w, height: cd.h,
    layout: 'vertical',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 6,
    fill: '#111321EB',
    cornerRadius: cd.rad,
    stroke: { fill: '#323A584D', thickness: 0.7 },
    children: [
      { type: 'icon_font', iconFontFamily: 'lucide', iconFontName: clientIcons[c.id], width: 24, height: 24, fill: '#67E8F9D9' },
      { type: 'text', content: c.label, fontFamily: 'Inter', fontSize: 12, fontWeight: '700', fill: '#FFFFFFEB' },
      { type: 'text', content: c.sub, fontFamily: 'Inter', fontSize: 10, fill: '#FFFFFF59' },
    ],
  });
}

for (let i = 0; i < GP.length; i++) {
  const g = GP[i];
  const gd = cards[g.id];
  nodes.push({
    type: 'frame',
    x: gd.x, y: gd.y, width: gd.w, height: gd.h,
    layout: 'horizontal',
    alignItems: 'center',
    gap: 10,
    padding: [0, 12],
    fill: '#111321EB',
    cornerRadius: gd.rad,
    stroke: { fill: '#323A584D', thickness: 0.7 },
    children: [
      { type: 'icon_font', iconFontFamily: 'lucide', iconFontName: 'cpu', width: 20, height: 20, fill: '#34D399D9' },
      {
        type: 'frame',
        layout: 'vertical',
        gap: 2,
        width: 'fit_content',
        height: 'fit_content',
        children: [
          { type: 'text', content: g.label, fontFamily: 'Inter', fontSize: 12, fontWeight: '700', fill: '#FFFFFFEB' },
          { type: 'text', content: g.sub, fontFamily: 'Inter', fontSize: 10, fill: '#FFFFFF59' },
        ],
      },
    ],
  });
}

const hb = cards.hub;
nodes.push({
  type: 'frame',
  x: hb.x, y: hb.y, width: hb.w, height: hb.h,
  layout: 'vertical',
  alignItems: 'center',
  justifyContent: 'center',
  gap: 6,
  fill: '#0F0F1EED',
  cornerRadius: hb.rad,
  stroke: { fill: '#694BC34D', thickness: 1 },
  children: [
    { type: 'icon_font', iconFontFamily: 'lucide', iconFontName: 'server', width: 28, height: 28, fill: '#A78BFA' },
    { type: 'text', content: 'cocompute', fontFamily: 'Inter', fontSize: 14, fontWeight: '700', fill: '#FFFFFFF2' },
    { type: 'text', content: 'orchestrator', fontFamily: 'Inter', fontSize: 10, fill: '#A78BFA8C' },
  ],
});

return nodes;

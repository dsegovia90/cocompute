(function(){
var sc=document.getElementById('network-sc');
if(!sc) return;
var cv=document.getElementById('network-cv');
var ctxC=cv.getContext('2d');
var ui=document.getElementById('network-ui');
var W=0,H=0,dpr=1,mobile=false;
var cards={};
var pths={};
var reqs=[];
var fr=0;
var colorIdx=0;

var COLORS=[
  {r:34,g:211,b:238},
  {r:192,g:132,b:252},
  {r:52,g:211,b:153}
];

var CL=[
  {id:'c0',label:'Desktop',sub:'workstation',icon:'desktop'},
  {id:'c1',label:'Mobile',sub:'smartphone',icon:'mobile'},
  {id:'c2',label:'Laptop',sub:'notebook',icon:'laptop'}
];
var GP=[
  {id:'g0',label:'Host GPU',sub:'RTX 4090'},
  {id:'g1',label:'Host GPU',sub:'A100 80GB'},
  {id:'g2',label:'Host GPU',sub:'RTX 3090'},
  {id:'g3',label:'Host GPU',sub:'H100 SXM'},
  {id:'g4',label:'Host GPU',sub:'RTX 4080'}
];

var svgDesktop='<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>';
var svgMobile='<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="2" width="14" height="20" rx="2"/><line x1="12" y1="18" x2="12.01" y2="18"/></svg>';
var svgLaptop='<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M20 16V7a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v9"/><rect x="1" y="16" width="22" height="4" rx="1"/></svg>';
var svgChip='<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6" rx="1"/><line x1="9" y1="1" x2="9" y2="4"/><line x1="15" y1="1" x2="15" y2="4"/><line x1="9" y1="20" x2="9" y2="23"/><line x1="15" y1="20" x2="15" y2="23"/><line x1="20" y1="9" x2="23" y2="9"/><line x1="20" y1="15" x2="23" y2="15"/><line x1="1" y1="9" x2="4" y2="9"/><line x1="1" y1="15" x2="4" y2="15"/></svg>';
var svgServer='<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="20" height="8" rx="2"/><rect x="2" y="14" width="20" height="8" rx="2"/><circle cx="6" cy="6" r="1" fill="currentColor" stroke="none"/><circle cx="6" cy="18" r="1" fill="currentColor" stroke="none"/><line x1="10" y1="6" x2="18" y2="6"/><line x1="10" y1="18" x2="18" y2="18"/></svg>';
var clientIcons={desktop:svgDesktop,mobile:svgMobile,laptop:svgLaptop};

function doInit(){
  var r=sc.getBoundingClientRect();
  dpr=window.devicePixelRatio||1;
  W=r.width; H=r.height;
  mobile=W<640;
  cv.width=W*dpr; cv.height=H*dpr;
  ctxC.setTransform(dpr,0,0,dpr,0,0);
  reqs=[];
  buildLayout();
}

function buildLayout(){
  ui.innerHTML='';
  cards={};
  pths={};

  if(mobile){
    buildMobile();
  } else {
    buildDesktop();
  }
  renderCards();
}

function buildDesktop(){
  var cw=Math.max(Math.min(W*0.115,96),80);
  var ch=Math.max(Math.min(H*0.2,88),80);
  var gpW=Math.max(Math.min(W*0.155,138),130);
  var gpH=Math.max(Math.min(H*0.112,50),48);
  var hubW=Math.max(Math.min(W*0.13,115),115);
  var hubH=Math.max(Math.min(H*0.24,108),100);
  var my=H*0.5;

  cards.hub={x:W*0.5-hubW/2,y:my-hubH/2,w:hubW,h:hubH,cx:W*0.5,cy:my,rad:16,glow:0,gc:null};

  for(var i=0;i<CL.length;i++){
    var sp=H*0.56, sy=my-sp/2;
    var yy=sy+sp*i/(CL.length-1)-ch/2;
    cards[CL[i].id]={x:W*0.06,y:yy,w:cw,h:ch,cx:W*0.06+cw/2,cy:yy+ch/2,rad:14,glow:0,gc:null};
  }
  for(var i=0;i<GP.length;i++){
    var sp=H*0.82, sy=my-sp/2;
    var yy=sy+sp*i/(GP.length-1)-gpH/2;
    var xx=W*0.94-gpW;
    cards[GP[i].id]={x:xx,y:yy,w:gpW,h:gpH,cx:xx+gpW/2,cy:yy+gpH/2,rad:12,glow:0,gc:null};
  }

  // horizontal bezier paths
  for(var i=0;i<CL.length;i++){
    var cd=cards[CL[i].id], hb=cards.hub;
    var sx=cd.x+cd.w, ex=hb.x;
    pths[CL[i].id+'_hub']=[sx,cd.cy, sx+(ex-sx)*0.5,cd.cy, sx+(ex-sx)*0.5,hb.cy, ex,hb.cy];
    pths['hub_'+CL[i].id]=[ex,hb.cy, sx+(ex-sx)*0.5,hb.cy, sx+(ex-sx)*0.5,cd.cy, sx,cd.cy];
  }
  for(var i=0;i<GP.length;i++){
    var gd=cards[GP[i].id], hb=cards.hub;
    var sx=hb.x+hb.w, ex=gd.x;
    pths['hub_'+GP[i].id]=[sx,hb.cy, sx+(ex-sx)*0.5,hb.cy, sx+(ex-sx)*0.5,gd.cy, ex,gd.cy];
    pths[GP[i].id+'_hub']=[ex,gd.cy, sx+(ex-sx)*0.5,gd.cy, sx+(ex-sx)*0.5,hb.cy, sx,hb.cy];
  }
}

function buildMobile(){
  var cw=Math.min(W*0.22,80);
  var ch=88;
  var gpW=Math.min(W*0.36,130);
  var gpH=56;
  var hubW=Math.min(W*0.32,115);
  var hubH=100;

  // clients row at top
  var clY=H*0.05;
  var clSpan=W*0.68;
  var clStart=(W-clSpan)/2;
  for(var i=0;i<CL.length;i++){
    var xx=clStart+clSpan*i/(CL.length-1)-cw/2;
    cards[CL[i].id]={x:xx,y:clY,w:cw,h:ch,cx:xx+cw/2,cy:clY+ch/2,rad:14,glow:0,gc:null};
  }

  // Equal gap between clients→hub and hub→GPUs
  var clientsBottom=clY+ch;
  var bottomPad=20; // margin at very bottom of canvas
  var gap=(H-clientsBottom-hubH-gpH-gpH*1.2-bottomPad)/3;
  var hubY=clientsBottom+gap;
  cards.hub={x:W*0.5-hubW/2,y:hubY,w:hubW,h:hubH,cx:W*0.5,cy:hubY+hubH/2,rad:16,glow:0,gc:null};

  // GPUs row at bottom — staggered: left/right higher, middle lower
  var gpY=hubY+hubH+gap;
  var gpStagger=gpH*1.2;
  var gpSpan=W-gpW-40;
  var gpStart=(W-gpSpan)/2;
  var gpCount=Math.min(GP.length,3);
  for(var i=0;i<gpCount;i++){
    var xx=gpStart+gpSpan*i/(gpCount-1)-gpW/2;
    var yy=gpY+(i===1?gpStagger:0);
    cards[GP[i].id]={x:xx,y:yy,w:gpW,h:gpH,cx:xx+gpW/2,cy:yy+gpH/2,rad:12,glow:0,gc:null};
  }

  // vertical bezier paths: clients -> hub (top to middle)
  for(var i=0;i<CL.length;i++){
    var cd=cards[CL[i].id], hb=cards.hub;
    var sy=cd.y+cd.h, ey=hb.y;
    pths[CL[i].id+'_hub']=[cd.cx,sy, cd.cx,sy+(ey-sy)*0.5, hb.cx,sy+(ey-sy)*0.5, hb.cx,ey];
    pths['hub_'+CL[i].id]=[hb.cx,ey, hb.cx,sy+(ey-sy)*0.5, cd.cx,sy+(ey-sy)*0.5, cd.cx,sy];
  }
  // hub -> GPUs (middle to bottom)
  for(var i=0;i<gpCount;i++){
    var gd=cards[GP[i].id], hb=cards.hub;
    var sy=hb.y+hb.h, ey=gd.y;
    pths['hub_'+GP[i].id]=[hb.cx,sy, hb.cx,sy+(ey-sy)*0.5, gd.cx,sy+(ey-sy)*0.5, gd.cx,ey];
    pths[GP[i].id+'_hub']=[gd.cx,ey, gd.cx,sy+(ey-sy)*0.5, hb.cx,sy+(ey-sy)*0.5, hb.cx,sy];
  }
}

function renderCards(){
  for(var i=0;i<CL.length;i++){
    var c=CL[i], cd=cards[c.id];
    if(!cd) continue;
    var d=document.createElement('div');
    d.style.cssText='position:absolute;left:'+cd.x+'px;top:'+cd.y+'px;width:'+cd.w+'px;height:'+cd.h+'px;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:6px';
    d.innerHTML='<div style="color:#67e8f9;opacity:0.85">'+clientIcons[c.icon]+'</div>'
      +'<div style="font-size:12px;font-weight:700;color:rgba(255,255,255,0.92)">'+c.label+'</div>'
      +'<div style="font-size:10px;font-weight:400;color:rgba(255,255,255,0.35)">'+c.sub+'</div>';
    ui.appendChild(d);
  }

  for(var i=0;i<GP.length;i++){
    var g=GP[i], gd=cards[g.id];
    if(!gd) continue;
    var d=document.createElement('div');
    d.style.cssText='position:absolute;left:'+gd.x+'px;top:'+gd.y+'px;width:'+gd.w+'px;height:'+gd.h+'px;display:flex;align-items:center;gap:10px;padding:0 12px';
    d.innerHTML='<div style="color:#34d399;opacity:0.85;flex-shrink:0">'+svgChip+'</div>'
      +'<div style="min-width:0">'
      +'<div style="font-size:12px;font-weight:700;color:rgba(255,255,255,0.92);white-space:nowrap">'+g.label+'</div>'
      +'<div style="font-size:10px;font-weight:400;color:rgba(255,255,255,0.35);white-space:nowrap">'+g.sub+'</div>'
      +'</div>';
    ui.appendChild(d);
  }

  var hb=cards.hub;
  var hd=document.createElement('div');
  hd.style.cssText='position:absolute;left:'+hb.x+'px;top:'+hb.y+'px;width:'+hb.w+'px;height:'+hb.h+'px;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:6px';
  hd.innerHTML='<div style="color:#a78bfa">'+svgServer+'</div>'
    +'<div style="font-size:14px;font-weight:700;color:rgba(255,255,255,0.95);letter-spacing:0.02em">Orchestrator</div>'
    +'<div style="font-size:10px;font-weight:400;color:#a78bfa;opacity:0.55">cocompute</div>';
  ui.appendChild(hd);
}

function bz(p,t){
  var u=1-t;
  return{
    x:u*u*u*p[0]+3*u*u*t*p[2]+3*u*t*t*p[4]+t*t*t*p[6],
    y:u*u*u*p[1]+3*u*u*t*p[3]+3*u*t*t*p[5]+t*t*t*p[7]
  };
}

function spawnReq(){
  var ci=Math.floor(Math.random()*CL.length);
  var gpCount=mobile?Math.min(GP.length,3):GP.length;
  var gi=Math.floor(Math.random()*gpCount);
  var col=COLORS[colorIdx%COLORS.length];
  colorIdx++;
  reqs.push({cid:CL[ci].id, gid:GP[gi].id, col:col, st:0, t:0, spd:0.006+Math.random()*0.003, dw:0, dwMax:0, trail:[], sz:2.2});
}

function pathFor(r){
  if(r.st===0) return pths[r.cid+'_hub'];
  if(r.st===2) return pths['hub_'+r.gid];
  if(r.st===4) return pths[r.gid+'_hub'];
  if(r.st===6) return pths['hub_'+r.cid];
  return null;
}

function stepSim(){
  var i,r,mv,p,pt;
  for(i=0;i<reqs.length;i++){
    r=reqs[i];
    if(r.st>=8) continue;
    mv=(r.st===0||r.st===2||r.st===4||r.st===6);
    if(mv){
      r.t+=r.spd;
      p=pathFor(r);
      if(p){
        pt=bz(p,Math.min(1,r.t));
        r.trail.push({x:pt.x,y:pt.y,a:1});
      }
      if(r.t>=1){
        r.t=0; r.st++; r.dw=0;
        if(r.st===1){r.dwMax=30+Math.random()*18|0; cards.hub.glow=1; cards.hub.gc=r.col;}
        if(r.st===3&&cards[r.gid]){r.dwMax=50+Math.random()*30|0; cards[r.gid].glow=1; cards[r.gid].gc=r.col;}
        if(r.st===5){r.dwMax=14+Math.random()*8|0; cards.hub.glow=Math.max(cards.hub.glow,0.8); cards.hub.gc=r.col;}
        if(r.st===7){cards[r.cid].glow=0.7; cards[r.cid].gc=r.col; r.dwMax=6;}
      }
    } else {
      r.dw++;
      if(r.st===1||r.st===5) cards.hub.glow=Math.max(cards.hub.glow, 0.3+0.6*Math.sin(r.dw*0.18));
      if(r.st===3&&cards[r.gid]) cards[r.gid].glow=Math.max(cards[r.gid].glow, 0.3+0.6*Math.sin(r.dw*0.14));
      if(r.dw>=r.dwMax){r.st++; r.t=0; r.trail=[];}
    }
  }
  for(i=0;i<reqs.length;i++){
    var tr=reqs[i].trail;
    for(var j=0;j<tr.length;j++) tr[j].a-=0.012;
    reqs[i].trail=tr.filter(function(p){return p.a>0;});
  }
  reqs=reqs.filter(function(r){return r.st<8;});
  cards.hub.glow*=0.96;
  for(i=0;i<CL.length;i++) if(cards[CL[i].id]) cards[CL[i].id].glow*=0.92;
  for(i=0;i<GP.length;i++) if(cards[GP[i].id]) cards[GP[i].id].glow*=0.92;
}

function rrPath(x,y,w,h,rad){
  ctxC.beginPath();
  ctxC.moveTo(x+rad,y);
  ctxC.lineTo(x+w-rad,y);
  ctxC.quadraticCurveTo(x+w,y,x+w,y+rad);
  ctxC.lineTo(x+w,y+h-rad);
  ctxC.quadraticCurveTo(x+w,y+h,x+w-rad,y+h);
  ctxC.lineTo(x+rad,y+h);
  ctxC.quadraticCurveTo(x,y+h,x,y+h-rad);
  ctxC.lineTo(x,y+rad);
  ctxC.quadraticCurveTo(x,y,x+rad,y);
  ctxC.closePath();
}

function drawGlow(cd,fallback){
  var g=cd.glow;
  if(g<0.06) return;
  var co=cd.gc||fallback;
  var rs='rgba('+co.r+','+co.g+','+co.b+',';
  rrPath(cd.x-10,cd.y-10,cd.w+20,cd.h+20,cd.rad+6);
  ctxC.strokeStyle=rs+(0.15*g)+')'; ctxC.lineWidth=1; ctxC.stroke();
  rrPath(cd.x-5,cd.y-5,cd.w+10,cd.h+10,cd.rad+3);
  ctxC.strokeStyle=rs+(0.35*g)+')'; ctxC.lineWidth=1.5; ctxC.stroke();
  rrPath(cd.x-1,cd.y-1,cd.w+2,cd.h+2,cd.rad+1);
  ctxC.strokeStyle=rs+(0.5*g)+')'; ctxC.lineWidth=1.5; ctxC.stroke();
  rrPath(cd.x,cd.y,cd.w,cd.h,cd.rad);
  ctxC.fillStyle=rs+(0.07*g)+')'; ctxC.fill();
}

function renderFrame(){
  ctxC.clearRect(0,0,W,H);

  ctxC.lineWidth=0.6;
  var keys=Object.keys(pths);
  for(var k=0;k<keys.length;k++){
    var p=pths[keys[k]];
    ctxC.beginPath();
    ctxC.moveTo(p[0],p[1]);
    ctxC.bezierCurveTo(p[2],p[3],p[4],p[5],p[6],p[7]);
    ctxC.strokeStyle='rgba(130,120,200,0.04)';
    ctxC.stroke();
  }

  var i;
  for(i=0;i<CL.length;i++) if(cards[CL[i].id]) drawGlow(cards[CL[i].id],COLORS[0]);
  for(i=0;i<GP.length;i++) if(cards[GP[i].id]) drawGlow(cards[GP[i].id],COLORS[2]);
  drawGlow(cards.hub,COLORS[1]);

  for(i=0;i<CL.length;i++){
    var cd=cards[CL[i].id];
    if(!cd) continue;
    rrPath(cd.x,cd.y,cd.w,cd.h,cd.rad);
    ctxC.fillStyle='rgba(17,19,33,0.92)'; ctxC.fill();
    ctxC.strokeStyle='rgba(50,58,88,'+(0.3+cd.glow*0.35)+')'; ctxC.lineWidth=0.7; ctxC.stroke();
  }
  for(i=0;i<GP.length;i++){
    var gd=cards[GP[i].id];
    if(!gd) continue;
    rrPath(gd.x,gd.y,gd.w,gd.h,gd.rad);
    ctxC.fillStyle='rgba(17,19,33,0.92)'; ctxC.fill();
    ctxC.strokeStyle='rgba(50,58,88,'+(0.3+gd.glow*0.35)+')'; ctxC.lineWidth=0.7; ctxC.stroke();
  }

  var hb=cards.hub;
  rrPath(hb.x,hb.y,hb.w,hb.h,hb.rad);
  ctxC.fillStyle='rgba(15,15,30,0.93)'; ctxC.fill();
  ctxC.strokeStyle='rgba(105,75,195,'+(0.3+hb.glow*0.4)+')'; ctxC.lineWidth=1; ctxC.stroke();

  for(i=0;i<reqs.length;i++){
    var r=reqs[i];
    var co=r.col;
    var rs='rgba('+co.r+','+co.g+','+co.b+',';
    var j,tr=r.trail;

    if(tr.length>1){
      for(j=1;j<tr.length;j++){
        var a0=tr[j-1].a, a1=tr[j].a;
        var avgA=(a0+a1)*0.5;
        ctxC.beginPath();
        ctxC.moveTo(tr[j-1].x,tr[j-1].y);
        ctxC.lineTo(tr[j].x,tr[j].y);
        ctxC.strokeStyle=rs+(avgA*0.5)+')';
        ctxC.lineWidth=r.sz*(0.3+avgA*0.7);
        ctxC.stroke();
      }
    }

    var mv=(r.st===0||r.st===2||r.st===4||r.st===6);
    if(mv && tr.length>0){
      var hd=tr[tr.length-1];
      var gr=ctxC.createRadialGradient(hd.x,hd.y,0,hd.x,hd.y,r.sz*6);
      gr.addColorStop(0,rs+'0.5)');
      gr.addColorStop(0.25,rs+'0.1)');
      gr.addColorStop(1,rs+'0)');
      ctxC.beginPath(); ctxC.arc(hd.x,hd.y,r.sz*6,0,6.283); ctxC.fillStyle=gr; ctxC.fill();
      ctxC.beginPath(); ctxC.arc(hd.x,hd.y,r.sz*0.7,0,6.283);
      ctxC.fillStyle='rgba(255,255,255,0.88)'; ctxC.fill();
    }
  }
}

function mainLoop(){
  stepSim();
  renderFrame();
  if(fr%120===0) spawnReq();
  fr++;
  requestAnimationFrame(mainLoop);
}

window.addEventListener('resize',doInit);
doInit();
mainLoop();
})();

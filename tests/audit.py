"""Audit automatique du simulateur : joue N matchs sans affichage et contrôle à chaque instant
l'intégrité de la grille, les règles (passes en avant, exclus) et le réalisme des statistiques.

Utilisation : pip install playwright && playwright install chromium
              python3 tests/audit.py 40        (CHROME=/chemin/vers/chrome pour un navigateur existant)
"""
import json, os, sys
from playwright.sync_api import sync_playwright
JS = r"""() => {
  const S = window.__sim; const ids=['TOU','UBB','MHR','RCT','SR','R92','ASM','SP','SF','AB','CO','LOU','USAP','RCV'];
  const A = {matches:0, ticks:0, viol:{}, hist:{}, sum:{}, ex:{}};
  const V = (k, ex) => { A.viol[k]=(A.viol[k]||0)+1; if(ex && !A.ex[k]) A.ex[k]=ex; };
  const H = (k, v) => { (A.hist[k]=A.hist[k]||[]).push(v); };
  const add = (k, v) => { A.sum[k]=(A.sum[k]||0)+v; };
  const N = %d;
  for(let i=0;i<N;i++){
    const h=ids[i%14], a=ids[(i*3+1+Math.floor(i/14))%14]===h?ids[(i+1)%14]:ids[(i*3+1+Math.floor(i/14))%14];
    const m = new S.Match(h,a,{headless:true});
    // espion passes en avant
    const op = m.passBall.bind(m);
    m.passBall = function(c, r, o){ const d=this.dir(c.team); if(d*(r.x-c.x) > .05) V('passe_en_avant_exécutée', `${c.num}->${r.num} dx=${(d*(r.x-c.x)).toFixed(2)}`); if(r.team!==c.team) V('passe_adversaire'); if(!this.active(r)) V('passe_vers_exclu'); return op(c,r,o); };
    let phaseLen=0, lastPhase='', ruckStart=0, looseLen=0, lastHalf=1, stillT=new Map();
    let maxOver=[0,0];
    while(!m.over){
      m.step(); A.ticks++;
      // intégrité grille
      const seen = new Map();
      for(const p of m.P){
        const act = m.active(p);
        if(!act){ if(p.c>=0) V('exclu_sur_terrain'); continue; }
        if(p.c<0 || p.r<0 || p.c>=43 || p.r>=29){ V('hors_grille', `${p.team}/${p.num} ${p.c},${p.r} ${m.phase}`); continue; }
        const k=p.r*43+p.c;
        if(seen.has(k)) V('deux_joueurs_meme_case', `${m.phase}`); seen.set(k,p);
        if(m.occ[k]!==p.id) V('occupation_incohérente', `${m.phase} ${p.team}/${p.num}`);
        if(Math.abs(p.x-(p.c+.5*(p.r&1)))>1e-6) V('coord_désynchro');
      }
      for(let k=0;k<m.occ.length;k++){ const id=m.occ[k]; if(id>=0){ const p=m.P[id]; if(!m.active(p) || p.r*43+p.c!==k) V('case_fantôme', m.phase); } }
      // ballon
      const b=m.ball;
      if(b.state==='carried'){ const c=b.carrier; if(!c) V('porteur_null'); else { if(!m.active(c)) V('porteur_exclu'); if(m.poss>=0 && c.team!==m.poss && m.phase!=='maul') V('porteur_pas_possession', `${m.phase} poss=${m.poss} t=${c.team}`); if(c.state==='ground' && m.phase==='open') V('porteur_au_sol_en_jeu'); } }
      if(b.state==='loose'){ looseLen++; } else { if(looseLen) H('ballon_au_sol_ticks', looseLen); looseLen=0; }
      if(!Number.isFinite(m.ballX())) V('ballon_position_NaN', m.phase);
      // durées de phase
      if(m.phase!==lastPhase){ if(lastPhase) H('phase_'+lastPhase, phaseLen); phaseLen=0; lastPhase=m.phase; } phaseLen++;
      // effectif sur le terrain
      for(const t of [0,1]){ const n=m.P.filter(p=>p.team===t&&m.active(p)).length; if(n<13) V('moins_de_13_joueurs', `${n}`); }
      // prolongation après sirène
      if(m.hooter()) maxOver[m.half-1]=Math.max(maxOver[m.half-1], m.halfTick-m.halfTicks);
      // hors-jeu au moment où le ballon sort du ruck
      // joueurs immobiles très loin de l'action en jeu courant
      if(m.phase==='open' && b.state==='carried'){
        for(const p of m.P){ if(!m.active(p)||p.state!=='up') continue; const key=p.id; const pos=p.c*100+p.r; const s=stillT.get(key);
          if(s && s.pos===pos){ s.n++; if(s.n===40 && Math.hypot(p.x-b.x,p.y-b.y)>10) V('joueur_figé_40_ticks_loin', `${p.team}/${p.num}`); } else stillT.set(key,{pos,n:0}); }
      } else stillT.clear();
    }
    H('prolong_mt1', maxOver[0]); H('prolong_mt2', maxOver[1]);
    A.matches++;
    const [x,y]=m.score; H('ecart', Math.abs(x-y)); H('total', x+y); if(x===y) add('nuls',1); if(x>y) add('vict_dom',1);
    for(const t of [0,1]){ const s=m.st[t];
      for(const k of ['tries','conv','convA','pg','pgA','drops','passes','offloads','breaks','tackles','missed','rucks','rucksWon','turnovers','pens','scrumFeeds','scrumWon','scrumSteals','lineouts','lineoutsWon','lineoutSteals','kicks','knock','intercepts','yellow','red','subs','inj','possT','terrT']) add(k, s[k]);
      for(const k in s.planUse) add('plan_'+k, s.planUse[k]); for(const k in s.defUse) add('def_'+k, s.defUse[k]);
    }
  }
  const out = {matches:A.matches, ticks_par_match:Math.round(A.ticks/A.matches), violations:A.viol, exemples:A.ex, par_match:{}, distrib:{}};
  for(const k in A.sum) out.par_match[k] = +(A.sum[k]/A.matches).toFixed(2);
  for(const k in A.hist){ const v=A.hist[k].slice().sort((p,q)=>p-q); out.distrib[k] = {n:v.length, moy:+(v.reduce((p,q)=>p+q,0)/v.length).toFixed(1), med:v[v.length>>1], p95:v[Math.floor(v.length*.95)], max:v[v.length-1]}; }
  return out;
}"""
with sync_playwright() as p:
    b = p.chromium.launch(**({"executable_path": os.environ["CHROME"]} if os.environ.get("CHROME") else {}), args=["--no-sandbox"])
    pg = b.new_page(); errs=[]; pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.goto("file://" + os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "index.html"))); pg.wait_for_timeout(500)
    r = pg.evaluate(JS.replace("%d", str(int(sys.argv[1]) if len(sys.argv) > 1 else 40)))
    print(json.dumps(r, ensure_ascii=False, indent=1)); print("erreurs page:", errs)
    b.close()

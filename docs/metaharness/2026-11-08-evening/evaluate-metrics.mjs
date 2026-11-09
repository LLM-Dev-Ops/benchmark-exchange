import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
const [base,head,tools,out]=process.argv.slice(2),hash=b=>createHash('sha256').update(b).digest('hex');
const fw=await import(pathToFileURL(tools+'/node_modules/@metaharness/flywheel/dist/index.js'));
assert.equal(JSON.parse(readFileSync(tools+'/node_modules/@metaharness/flywheel/package.json')).version,'0.1.12');
for(const path of ['functions/agents/test.js','functions/agents/metric-probe.cjs','docs/metaharness/2026-11-08-evening/contract.json'])assert.ok(readFileSync(base+'/'+path).equals(readFileSync(head+'/'+path)),'Frozen bytes changed:'+path);
function measure(root){const r=spawnSync(process.execPath,['functions/agents/metric-probe.cjs','.'],{cwd:root,encoding:'utf8',timeout:10000});assert.ok([0,1].includes(r.status));return JSON.parse(r.stdout);}
const trials=[];for(let i=0;i<3;i++)for(const root of i%2?[head,base]:[base,head])trials.push({i,root,report:measure(root)});
const inverse=['functions/agents/index.js','functions/agents/package.json'].map(path=>({path,parentBytes:readFileSync(base+'/'+path,'utf8'),sha256:hash(readFileSync(base+'/'+path))}));
const measurements=[];
const result=await fw.runFlywheelGenerations({rootPolicy:{revision:hash(readFileSync(base+'/functions/agents/index.js'))},proposer:async()=>({value:hash(readFileSync(head+'/functions/agents/index.js')),summary:'Validate finite numeric metric entries',inverse}),evaluator:async(policy,suite)=>{const root=policy.revision===hash(readFileSync(base+'/functions/agents/index.js'))?base:head,report=measure(root),items=report[suite.id==='holdout'?'holdout':'anchors'],wins=items.map(x=>x.passed),score={primary:wins.filter(Boolean).length/wins.length,itemWins:wins,noopRate:0,costPerWin:0,regressed:true};measurements.push({root,suite:suite.id,report,score});return score;},holdout:{id:'holdout',items:[0,1,2,3,4,5,6,7]},anchor:{id:'anchors',items:[0,1,2,3,4]},maxGenerations:1,signer:fw.makeSigner(),promotionRule:fw.meetsPromotionRule,dataSource:'LIVE_NATIVE_HTTP_EXPORTED_HANDLER'});
const fingerprint=fw.gateFingerprint(fw.meetsPromotionRule),replay=fw.verifyReplayBundle(result.replayBundle,{pinnedGateFingerprint:fingerprint});assert.equal(replay.pass,true);assert.equal(result.promotions.length,0);
writeFileSync(out,JSON.stringify({trials,measurements,result,replay,fingerprint,inverse,contract:JSON.parse(readFileSync(base+'/docs/metaharness/2026-11-08-evening/contract.json')),boundary:'regressed=true is an explicit fail-closed veto for missing repository Rust lock/security/orchestration gates, not newly measured component regression. cost/noop0 neutral and unmeasured; root-only replay is not candidate promotion proof.'},null,2));console.log(JSON.stringify({trials:trials.map(x=>({i:x.i,root:x.root,holdout:x.report.holdout.filter(x=>x.passed).length,anchors:x.report.anchors.filter(x=>x.passed).length})),promotions:0,replay}));

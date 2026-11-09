'use strict';
const {createServer}=require('node:http');
const {resolve}=require('node:path');
function request(){return {benchmark_id:'bench-1',model_provider:'test',model_name:'model',model_version:'1',aggregate_score:0.5,metric_scores:{accuracy:{value:0.5}},methodology:{framework:'native',evaluation_method:'heldout',scoring_method:'exact'},dataset:{dataset_id:'test',dataset_version:'1',example_count:10,split:'test'},sample_size:10,variance:0.1,reproduction_count:1};}
async function probe(root){
 const {handler}=require(resolve(root,'functions/agents/index.js'));
 const server=createServer(async(req,res)=>{
  const chunks=[];for await(const chunk of req)chunks.push(chunk);req.body=chunks.length?JSON.parse(Buffer.concat(chunks)):undefined;req.path=new URL(req.url,'http://localhost').pathname;
  res.status=code=>{res.statusCode=code;return res;};res.json=body=>res.end(JSON.stringify(body));res.send=body=>res.end(body);res.set=(key,value)=>{res.setHeader(key,value);return res;};
  try{handler(req,res);}catch(error){res.statusCode=500;res.end(JSON.stringify({exception:error.name}));}
 });await new Promise(ok=>server.listen(0,'127.0.0.1',ok));
 async function check(name,metric,status,raw){const body=request();body.metric_scores=metric;
  const response=await fetch('http://127.0.0.1:'+server.address().port+'/v1/benchmark-exchange/publish',{method:'POST',headers:{'content-type':'application/json'},body:raw||JSON.stringify(body),signal:AbortSignal.timeout(5000)});const result=await response.json();
  return {name,status:response.status,passed:response.status===status&&(status===400?result.success===false&&result.error?.code==='VALIDATION_ERROR'&&result.error.details.some(x=>x.field==='metric_scores.accuracy.value'):result.success===true&&result.data.publication.status==='draft'&&/^[a-f0-9]{64}$/.test(result.data.decision_event.inputs_hash))};
 }
 try{const invalid=[null,1,[],{}, {value:'0.5'},{value:true},{value:null}];const holdout=[];for(let i=0;i<invalid.length;i++)holdout.push(await check('invalid-metric-'+i,{accuracy:invalid[i]},400));const overflow=JSON.stringify(request()).replace('"value":0.5','"value":1e400');holdout.push(await check('overflow',{},400,overflow));
 const anchors=[];for(const [name,metric]of [['zero',{accuracy:{value:0}}],['normal',{accuracy:{value:0.5}}],['signed',{accuracy:{value:-2}}],['latency',{accuracy:{value:5000,unit:'ms',higher_is_better:false}}],['empty',{}]])anchors.push(await check(name,metric,201));return {holdout,anchors};}
 finally{server.closeAllConnections();await new Promise(ok=>server.close(ok));}
}
module.exports={probe};
if(require.main===module)probe(process.argv[2]||'../..').then(r=>{console.log(JSON.stringify(r));if([...r.holdout,...r.anchors].some(x=>!x.passed))process.exitCode=1;});

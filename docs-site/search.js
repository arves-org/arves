(function(){
  var up='../'.repeat(window.DEPTH||0);
  var q=document.getElementById('q'),box=document.getElementById('results'),idx=[];
  fetch(up+'search-index.json').then(function(r){return r.json()}).then(function(d){idx=d}).catch(function(){});
  function esc(s){return s.replace(/[&<>]/g,function(c){return{'&':'&amp;','<':'&lt;','>':'&gt;'}[c]})}
  q&&q.addEventListener('input',function(){
    var v=q.value.trim().toLowerCase();
    if(v.length<2){box.style.display='none';return}
    var hits=idx.map(function(p){var t=(p.title.toLowerCase().indexOf(v)>=0?3:0)+(p.text.toLowerCase().indexOf(v)>=0?1:0);return{p:p,s:t}}).filter(function(h){return h.s>0}).sort(function(a,b){return b.s-a.s}).slice(0,12);
    if(!hits.length){box.style.display='none';return}
    box.innerHTML=hits.map(function(h){return '<a href="'+up+h.p.url+'"><span class="t">'+esc(h.p.title)+'</span><br><span class="x">'+esc(h.p.text.slice(0,110))+'…</span></a>'}).join('');
    box.style.display='block';
  });
  document.addEventListener('click',function(e){if(!box.contains(e.target)&&e.target!==q)box.style.display='none'});
})();
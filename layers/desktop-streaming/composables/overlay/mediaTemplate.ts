/** A shared media definition used by editor, rehearsal and published output. */
export function mediaTemplate(kind: 'image' | 'video'): string {
  return String.raw`<div class="overlay-media"><${kind === 'image' ? 'img' : 'video muted playsinline'} referrerpolicy="no-referrer" hidden></${kind === 'image' ? 'img' : 'video'}><span class="media-status">Set a ${kind} source</span></div>
  <style>.overlay-media{width:100%;height:100%;position:relative;overflow:hidden;border-radius:var(--c-radius);color:var(--c-muted);font:13px var(--theme-font-primary);display:grid;place-items:center}.overlay-media img,.overlay-media video{width:100%;height:100%;position:absolute;inset:0}.media-status{padding:12px;overflow-wrap:anywhere;text-align:center}</style>
  <script>(()=>{
    const root=document.currentScript.previousElementSibling.previousElementSibling;
    const media=root.querySelector('${kind === 'image' ? 'img' : 'video'}'),status=root.querySelector('.media-status');let data={},source='';
    function fail(message){media.hidden=true;status.hidden=false;status.textContent=message}
    media.addEventListener('${kind === 'image' ? 'load' : 'loadeddata'}',()=>{media.hidden=false;status.hidden=true});
    media.addEventListener('error',()=>fail('Media could not be loaded. Check the source and its availability.'));
    function update(value){data={...data,...value};const next=String(data.src||'').trim();media.style.objectFit=['contain','cover','fill'].includes(data.objectFit)?data.objectFit:'contain';
      ${kind === 'image' ? "media.alt=String(data.alt||'');" : "media.muted=true;media.loop=data.loop!=='off';"}
      if(next===source)return;source=next;${kind === 'video' ? 'media.pause();' : ''}media.removeAttribute('src');
      if(!next){fail('Set a ${kind} source');return}
      const allowed=/^https:\/\//i.test(next)||${kind === 'image' ? String.raw`/^data:image\/(png|jpeg|gif|webp);base64,/i.test(next)` : 'false'};
      if(!allowed){fail('Use an HTTPS ${kind} URL${kind === 'image' ? ' or an embedded image' : ''}.');return}
      fail('Loading ${kind}…');media.src=next;${kind === 'video' ? "const play=media.play();if(play&&play.catch)play.catch(()=>fail('Video playback failed. Check the source and codec.'));" : ''}
    }
    update(window.vortideckData);window.addEventListener('vortideck-data-update',event=>update(event.detail));
  })();${'<' + '/script>'}`
}

export function mediaData(element: { schemaId?: string, data?: Record<string, unknown>, src?: unknown, imageUrl?: unknown }, defaults: Record<string, unknown> = {}): Record<string, unknown> {
  const data = { ...defaults, ...element.data }
  if ((element.schemaId === 'image' || element.schemaId === 'video') && !Object.hasOwn(element.data || {}, 'src')) {
    data.src = element.src || element.imageUrl || ''
  }
  return data
}

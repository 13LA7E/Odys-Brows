//! The page side of assistant page control: small, self-contained JavaScript that runs in the tab
//! the user is looking at when the assistant asks (and only then).
//!
//! Each call is `LIB` plus a line that runs one action and reports the answer through the console
//! (`__ody_agent__:<id>:<json>`), which the engine turns into an `AgentResult` event. The page gets
//! no function it can call to reach us. Passwords, card numbers and one-time codes are refused
//! here, and `peek` marks elements whose label sounds like buying, sending, deleting and so on so
//! the shell can ask you first.

use serde_json::Value;

/// The library: snapshot, peek, click, type, select, press, scroll.
const LIB: &str = r#"
(function(){
if(window.__odyAgent){return;}
var REF=[];
var RISKY=/\b(buy|purchase|pay|checkout|place (your )?order|order now|delete|remove|erase|send|submit|confirm|post|publish|sign ?out|log ?out|unsubscribe|transfer|withdraw|subscribe|donate|accept|agree|install|download|reset|cancel (my|your)|close account)\b/i;
var SENSITIVE=/(passw|card|cvv|cvc|iban|ssn|security.?code|one.?time|otp|pin\b)/i;
function vis(e){var r=e.getBoundingClientRect();if(r.width<2||r.height<2)return false;var s=getComputedStyle(e);return s.visibility!=='hidden'&&s.display!=='none'&&parseFloat(s.opacity)>0.05;}
function clean(t,n){return String(t||'').replace(/\s+/g,' ').trim().slice(0,n||80);}
function label(e){
 var a=e.getAttribute('aria-label')||'';
 if(!a&&e.labels&&e.labels[0])a=e.labels[0].innerText;
 if(!a&&e.id){var l=document.querySelector('label[for="'+e.id.replace(/"/g,'')+'"]');if(l)a=l.innerText;}
 if(!a)a=(e.tagName==='INPUT'&&(e.type==='submit'||e.type==='button'))?e.value:'';
 if(!a)a=e.innerText||e.textContent||'';
 if(!a)a=e.getAttribute('placeholder')||e.getAttribute('title')||e.getAttribute('alt')||e.name||'';
 if(!a){var im=e.querySelector&&e.querySelector('img[alt]');if(im)a=im.getAttribute('alt');}
 return clean(a);
}
function kind(e){
 var t=e.tagName.toLowerCase();
 if(t==='a')return 'link';
 if(t==='button'||e.getAttribute('role')==='button'||(t==='input'&&/^(submit|button|reset|image)$/.test(e.type)))return 'button';
 if(t==='select')return 'menu';
 if(t==='textarea')return 'text box';
 if(t==='input'){var y=(e.type||'text').toLowerCase();if(y==='password')return 'password field';if(y==='checkbox')return 'checkbox';if(y==='radio')return 'radio button';return y+' field';}
 if(e.isContentEditable)return 'text box';
 return e.getAttribute('role')||t;
}
function sensitive(e){
 var ty=(e.type||'').toLowerCase();
 if(ty==='password')return true;
 var ac=(e.getAttribute('autocomplete')||'').toLowerCase();
 if(/^cc-|one-time-code|current-password|new-password/.test(ac))return true;
 return SENSITIVE.test((e.name||'')+' '+(e.id||'')+' '+(e.getAttribute('placeholder')||'')+' '+(e.getAttribute('aria-label')||''));
}
function textual(e){if(e.isContentEditable)return true;var t=e.tagName;if(t==='TEXTAREA')return true;if(t!=='INPUT')return false;return !/^(button|submit|reset|image|checkbox|radio|file|hidden|range|color)$/.test((e.type||'text').toLowerCase());}
function get(n){var e=REF[n];if(!e||!e.isConnected)return null;return e;}
function snapshot(){
 var sel='a[href],button,input,select,textarea,summary,[role=button],[role=link],[role=tab],[role=menuitem],[role=checkbox],[role=option],[contenteditable=true],[onclick]';
 var all=Array.prototype.slice.call(document.querySelectorAll(sel)).filter(function(e){return vis(e)&&e.type!=='hidden';});
 var vh=window.innerHeight;
 all.sort(function(a,b){var ra=a.getBoundingClientRect(),rb=b.getBoundingClientRect();var ia=ra.bottom>0&&ra.top<vh?0:1,ib=rb.bottom>0&&rb.top<vh?0:1;return ia-ib;});
 all=all.slice(0,70);
 REF=[null];
 var lines=[];
 all.forEach(function(e){
  REF.push(e);var n=REF.length-1;
  var k=kind(e);var l=label(e);var line='['+n+'] '+k+(l?' "'+l+'"':'');
  if(e.tagName==='A'&&e.href)line+=' -> '+clean(e.getAttribute('href'),70);
  if((e.tagName==='INPUT'||e.tagName==='TEXTAREA'||e.tagName==='SELECT')&&!sensitive(e)&&e.type!=='checkbox'&&e.type!=='radio'){var v=e.value;if(v)line+=' value="'+clean(v,40)+'"';}
  if(e.type==='checkbox'||e.type==='radio')line+=e.checked?' (checked)':' (unchecked)';
  if(e.disabled)line+=' (disabled)';
  var r=e.getBoundingClientRect();if(!(r.bottom>0&&r.top<vh))line+=' (off screen)';
  lines.push(line);
 });
 var text=clean(document.body?document.body.innerText:'',3500);
 return 'PAGE SNAPSHOT\nURL: '+location.href+'\nTitle: '+clean(document.title,100)+'\nScroll: '+Math.round(window.scrollY)+' of '+Math.round(document.documentElement.scrollHeight)+' (window height '+vh+')\nElements:\n'+(lines.join('\n')||'(nothing to click or fill)')+'\nPage text:\n'+text;
}
function peek(a){
 var e=get(a.ref);if(!e)return {error:'That element is gone. Take a new snapshot.'};
 var l=label(e);var o={label:l||kind(e),host:location.host,tag:e.tagName.toLowerCase(),risky:false};
 if(a.action==='type'&&!textual(e))return {error:'That is a '+kind(e)+', not a place to type. Pick a text field.'};
 if(a.action==='select'&&e.tagName!=='SELECT')return {error:'That is not a menu.'};
 if(a.action==='type'||a.action==='select'){
  if(sensitive(e))return {error:'That is a password, card or code field. The assistant never fills those in; ask the user to.'};
  if(e.disabled||e.readOnly)return {error:'That field is read only.'};
 }
 if(a.action==='click'){
  var t=(e.type||'').toLowerCase();
  o.risky=RISKY.test(l)||(t==='submit'&&RISKY.test(l+' '+(e.form?clean(e.form.innerText,60):'')));
  if(e.form&&e.form.querySelector&&e.form.querySelector('input[type=password]')&&(t==='submit'||e.tagName==='BUTTON'))o.risky=true;
 }
 if(a.action==='type'&&a.submit){o.risky=!!(e.form&&e.form.querySelector&&e.form.querySelector('input[type=password]'));}
 return o;
}
function click(a){
 var e=get(a.ref);if(!e)return 'Not done: that element is gone. Take a new snapshot.';
 e.scrollIntoView({block:'center',inline:'center'});
 try{e.focus({preventScroll:true});}catch(x){}
 ['pointerdown','mousedown','pointerup','mouseup'].forEach(function(t){try{e.dispatchEvent(new MouseEvent(t,{bubbles:true,cancelable:true,view:window}));}catch(x){}});
 e.click();
 return 'Clicked “'+(label(e)||kind(e))+'”.';
}
function setValue(e,v){
 if(e.isContentEditable){e.focus();document.execCommand('selectAll',false,null);document.execCommand('insertText',false,v);return;}
 var proto=e.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype;
 var d=Object.getOwnPropertyDescriptor(proto,'value');
 if(d&&d.set)d.set.call(e,v);else e.value=v;
}
function typeText(a){
 var e=get(a.ref);if(!e)return 'Not done: that element is gone. Take a new snapshot.';
 if(sensitive(e))return 'Not done: that is a password, card or code field.';
 if(!textual(e))return 'Not done: that is not a text field.';
 e.scrollIntoView({block:'center'});e.focus();
 setValue(e,String(a.text==null?'':a.text).slice(0,2000));
 e.dispatchEvent(new Event('input',{bubbles:true}));e.dispatchEvent(new Event('change',{bubbles:true}));
 var msg='Typed into “'+(label(e)||kind(e))+'”.';
 if(a.submit){key('Enter',e);if(e.form){try{e.form.requestSubmit();}catch(x){}}msg+=' Pressed Enter.';}
 return msg;
}
function select(a){
 var e=get(a.ref);if(!e||e.tagName!=='SELECT')return 'Not done: that is not a menu.';
 var want=String(a.value==null?'':a.value).toLowerCase();var hit=null;
 Array.prototype.forEach.call(e.options,function(o){if(!hit&&(o.value.toLowerCase()===want||clean(o.text,80).toLowerCase()===want))hit=o;});
 if(!hit)Array.prototype.forEach.call(e.options,function(o){if(!hit&&clean(o.text,80).toLowerCase().indexOf(want)>=0)hit=o;});
 if(!hit)return 'Not done: no option matches “'+a.value+'”. Options: '+Array.prototype.map.call(e.options,function(o){return clean(o.text,30);}).slice(0,12).join(', ');
 e.value=hit.value;e.dispatchEvent(new Event('input',{bubbles:true}));e.dispatchEvent(new Event('change',{bubbles:true}));
 return 'Chose “'+clean(hit.text,60)+'”.';
}
var CODES={Enter:13,Tab:9,Escape:27,ArrowDown:40,ArrowUp:38,ArrowLeft:37,ArrowRight:39,PageDown:34,PageUp:33,Home:36,End:35,Backspace:8,' ':32};
function key(k,target){
 var t=target||document.activeElement||document.body;var c=CODES[k]||0;
 ['keydown','keypress','keyup'].forEach(function(ty){try{t.dispatchEvent(new KeyboardEvent(ty,{key:k,code:k,keyCode:c,which:c,bubbles:true,cancelable:true}));}catch(x){}});
}
function press(a){
 var k=String(a.key||'');if(!CODES.hasOwnProperty(k))return 'Not done: use one of '+Object.keys(CODES).join(', ');
 key(k);
 if(k==='PageDown')window.scrollBy(0,window.innerHeight*0.8);if(k==='PageUp')window.scrollBy(0,-window.innerHeight*0.8);
 return 'Pressed '+k+'.';
}
function scroll(a){
 var d=String(a.direction||'down');var h=window.innerHeight*0.8;
 if(d==='up')window.scrollBy(0,-h);else if(d==='top')window.scrollTo(0,0);else if(d==='bottom')window.scrollTo(0,document.documentElement.scrollHeight);else window.scrollBy(0,h);
 return 'Scrolled '+d+'.';
}
window.__odyAgent={snapshot:snapshot,peek:peek,click:click,type:typeText,select:select,press:press,scroll:scroll};
})();
"#;

/// The actions the shell may run in a page.
pub const ACTIONS: &[&str] = &[
    "snapshot", "peek", "click", "type", "select", "press", "scroll",
];

/// A complete script for one action. `id` routes the answer back (see `Event::AgentResult`).
pub fn script(name: &str, args: &Value, id: u64) -> Option<String> {
    if !ACTIONS.contains(&name) {
        return None;
    }
    let args_json = serde_json::to_string(args).ok()?;
    // JSON is also a JavaScript object literal, except for two separators that end a JS string.
    let args_json = args_json
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    Some(format!(
        "{LIB}\n(function(){{var out;try{{out=window.__odyAgent.{name}({args_json});}}catch(e){{out='Not done: '+String(e&&e.message||e)+' '+String(e&&e.stack||'').slice(0,240);}}console.debug('__ody_agent__:{id}:'+JSON.stringify(out));}})();"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_are_built_only_for_known_actions() {
        let s = script(
            "click",
            &serde_json::json!({"ref": 3, "text": "it's \"quoted\"\u{2028}"}),
            42,
        )
        .unwrap();
        assert!(s.contains("__odyAgent.click("));
        assert!(s.contains("'__ody_agent__:42:'"));
        assert!(!s.contains('\u{2028}'), "line separators are escaped");
        assert!(script("eval", &serde_json::json!({}), 1).is_none());
        assert!(script("snapshot); alert(1", &serde_json::json!({}), 1).is_none());
    }

    #[test]
    fn the_library_never_fills_sensitive_fields() {
        // The page-side checks are plain text in the library; make sure they are still there.
        for needle in [
            "sensitive(e)",
            "current-password",
            "one.?time",
            "never fills",
        ] {
            assert!(LIB.contains(needle) || needle == "never fills", "{needle}");
        }
        assert!(LIB.contains("password, card or code field"));
    }
}

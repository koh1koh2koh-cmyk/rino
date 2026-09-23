
function عدد(v) { const n = parseFloat(v); return isNaN(n) ? 0 : n; }
function نص(v) { return String(v); }
function اقرأ(id) {
  const el = document.getElementById(id);
  return el ? el.value : "";
}
function امسح(id) {
  const el = document.getElementById(id);
  if (el) el.innerHTML = "";
}
function أضف_مهمة(id_قائمة, نص) {
  const ul = document.getElementById(id_قائمة);
  if (!ul) return;
  const li = document.createElement("li");
  li.textContent = نص;
  li.style.padding = "10px";
  li.style.marginTop = "5px";
  li.style.background = "rgb(240, 240, 240)";
  li.style.borderRadius = "5px";
  li.style.cursor = "pointer";
  li.title = "انقر للحذف";
  li.onclick = function() { li.remove(); };
  ul.appendChild(li);
}

const حالة = new Proxy({  إعجابات: 0,
  زائر: "",
  رسالة: "",
}, {
  set(target, key, value) {
    target[key] = value;
    updateAll();
    return true;
  }
});

document.getElementById('r2').addEventListener('click', function() {
  حالة.إعجابات = (حالة.إعجابات + 1);
});
document.getElementById('r3').addEventListener('click', function() {
  حالة.إعجابات = (حالة.إعجابات - 1);
});
document.getElementById('r4').addEventListener('click', function() {
  حالة.إعجابات = 0;
});
document.getElementById('r5').addEventListener('click', function() {
  حالة.زائر = اقرأ("حقل_الاسم");
  حالة.رسالة = اقرأ("حقل_الرسالة");
  console.log(("رسالة من: " + حالة.زائر));
  console.log(("الرسالة: " + حالة.رسالة));
});
document.getElementById('r6').addEventListener('click', function() {
  حالة.إعجابات = (حالة.إعجابات + 10);
});


function updateAll() {
  document.getElementById('r1').textContent = ("عدد الإعجابات: " + حالة.إعجابات);
}

updateAll();

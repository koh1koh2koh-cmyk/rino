
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
function مفاتيح(d) { return Object.keys(d); }
function قيم(d) { return Object.values(d); }
function يحتوي_مفتاح(d, k) { return k in d; }

const حالة = new Proxy({  عدد_السلة: 0,
}, {
  set(target, key, value) {
    target[key] = value;
    updateAll();
    return true;
  }
});

document.getElementById('r1').addEventListener('click', function() {
  حالة.عدد_السلة = (حالة.عدد_السلة + 1);
});
document.getElementById('r2').addEventListener('click', function() {
  حالة.عدد_السلة = (حالة.عدد_السلة + 1);
});
document.getElementById('r3').addEventListener('click', function() {
  حالة.عدد_السلة = (حالة.عدد_السلة + 1);
});
document.getElementById('r4').addEventListener('click', function() {
  حالة.عدد_السلة = 0;
});


function updateAll() {
  document.getElementById('').textContent = (("السلة: " + حالة.عدد_السلة) + " منتج");
}

updateAll();

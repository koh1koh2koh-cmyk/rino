
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
function احفظ(مفتاح, قيمة) {
  try {
    localStorage.setItem(String(مفتاح), JSON.stringify(قيمة));
  } catch (e) {
    console.error("فشل الحفظ:", e);
  }
}
function اقرأ_محلي(مفتاح) {
  try {
    const v = localStorage.getItem(String(مفتاح));
    return v !== null ? JSON.parse(v) : null;
  } catch (e) {
    return null;
  }
}

const حالة = new Proxy({  الحكمة: "جاري التحميل...",
  العداد: 0,
}, {
  set(target, key, value) {
    target[key] = value;
    updateAll();
    return true;
  }
});

  fetch("https://catfact.ninja/fact")
    .then(function(r) { return r.json(); })
    .then(function(d) { if (typeof d === 'object' && d !== null) { const keys = Object.keys(d); for (const k of keys) { if (typeof d[k] === 'string') { حالة.الحكمة = d[k]; return; } } } حالة.الحكمة = (typeof d === 'string') ? d : JSON.stringify(d); })
    .catch(function(e) { حالة.الحكمة = 'خطأ: ' + e.message; });
document.getElementById('r2').addEventListener('click', function() {
  حالة.العداد = (حالة.العداد + 1);
  fetch("https://catfact.ninja/fact")
    .then(function(r) { return r.json(); })
    .then(function(d) { if (typeof d === 'object' && d !== null) { const keys = Object.keys(d); for (const k of keys) { if (typeof d[k] === 'string') { حالة.الحكمة = d[k]; return; } } } حالة.الحكمة = (typeof d === 'string') ? d : JSON.stringify(d); })
    .catch(function(e) { حالة.الحكمة = 'خطأ: ' + e.message; });
});


function updateAll() {
  document.getElementById('نص_الحكمة').textContent = حالة.الحكمة;
  document.getElementById('r1').textContent = ("عدد التحديثات: " + حالة.العداد);
}

updateAll();

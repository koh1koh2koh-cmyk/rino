
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
function أضف(قائمة, قيمة) {
  const l = قائمة.slice();
  l.push(قيمة);
  return l;
}
function احذف(قائمة, قيمة) {
  const l = قائمة.slice();
  const idx = l.indexOf(قيمة);
  if (idx >= 0) l.splice(idx, 1);
  return l;
}
function اعكس(قائمة) { return قائمة.slice().reverse(); }
function دمج(قائمة, فاصل) { return قائمة.join(فاصل === undefined ? "" : فاصل); }
function أول(قائمة) { return قائمة[0]; }
function آخر(قائمة) { return قائمة[قائمة.length - 1]; }
function مفهرس(قائمة, فهرس) { return قائمة[فهرس]; }
function يحتوي_قائمة(قائمة, قيمة) { return قائمة.indexOf(قيمة) >= 0; }
function رتب(قائمة) { return قائمة.slice().sort((a, b) => (typeof a === "number" && typeof b === "number") ? a - b : String(a).localeCompare(String(b), "ar")); }
function مدى(من, إلى) {
  const arr = [];
  if (من <= إلى) { for (let i = من; i <= إلى; i++) arr.push(i); }
  else { for (let i = من; i >= إلى; i--) arr.push(i); }
  return arr;
}

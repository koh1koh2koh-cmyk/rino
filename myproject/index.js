
function escape_html(s) {
  return String(s)
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}
function عدد(v) { const n = parseFloat(v); return isNaN(n) ? 0 : n; }
function نص(v) { return String(v); }
function سلسلة(v) { return String(v); }
function منطقي(v) { return Boolean(v); }
function تحقق_صحة(v) { return Boolean(v); }
function رقم_صحيح(v) { return Math.trunc(Number(v) || 0); }
function رقم_عشري(v) { return Number(v) || 0; }
function نوع(v) {
  if (Array.isArray(v)) return "قائمة";
  if (v === null) return "لا_شيء";
  const t = typeof v;
  if (t === "string") return "نص";
  if (t === "number") return "عدد";
  if (t === "boolean") return "منطقي";
  if (t === "object") return "قاموس";
  return t;
}
function اقرأ(id) { const el = document.getElementById(id); return el ? el.value : ""; }
function امسح(id) { const el = document.getElementById(id); if (el) el.innerHTML = ""; }
function أضف_مهمة(id_قائمة, نص) {
  const ul = document.getElementById(id_قائمة);
  if (!ul) return;
  const li = document.createElement("li");
  li.textContent = نص;
  li.style.padding = "10px"; li.style.marginTop = "5px";
  li.style.background = "rgb(240, 240, 240)";
  li.style.borderRadius = "5px"; li.style.cursor = "pointer";
  li.title = "انقر للحذف";
  li.onclick = function() { li.remove(); };
  ul.appendChild(li);
}
function مفاتيح(d) { return Object.keys(d); }
function قيم(d) { return Object.values(d); }
function يحتوي_مفتاح(d, k) { return k in d; }
function احفظ(مفتاح, قيمة) { try { localStorage.setItem(String(مفتاح), JSON.stringify(قيمة)); } catch (e) {} }
function اقرأ_محلي(مفتاح) { try { const v = localStorage.getItem(String(مفتاح)); return v !== null ? JSON.parse(v) : null; } catch (e) { return null; } }
function أضف(قائمة, قيمة) { const l = قائمة.slice(); l.push(قيمة); return l; }
function أضف_أمام(قائمة, قيمة) { const l = قائمة.slice(); l.unshift(قيمة); return l; }
function أدرج(قائمة, فهرس, قيمة) { const l = قائمة.slice(); l.splice(فهرس, 0, قيمة); return l; }
function احذف(قائمة, قيمة) { const l = قائمة.slice(); const idx = l.indexOf(قيمة); if (idx >= 0) l.splice(idx, 1); return l; }
function احذف_أخير(قائمة) { const l = قائمة.slice(); l.pop(); return l; }
function اعكس(قائمة) { return قائمة.slice().reverse(); }
function اقلب(قائمة) { return قائمة.slice().reverse(); }
function دمج(قائمة, فاصل) { return قائمة.join(فاصل === undefined ? "" : فاصل); }
function أول(قائمة) { return قائمة[0]; }
function آخر(قائمة) { return قائمة[قائمة.length - 1]; }
function مفهرس(قائمة, فهرس) { const i = فهرس < 0 ? قائمة.length + فهرس : فهرس; return قائمة[i]; }
function يحتوي_قائمة(قائمة, قيمة) { return قائمة.indexOf(قيمة) >= 0; }
function تضمن(قائمة, قيمة) { return قائمة.indexOf(قيمة) >= 0; }
function رتب(قائمة) { return قائمة.slice().sort((a, b) => (typeof a === "number" && typeof b === "number") ? a - b : String(a).localeCompare(String(b), "ar")); }
function فرز(قائمة) { return رتب(قائمة); }
function ترتيب(قائمة) { return رتب(قائمة); }
function فرق(قائمة) { return Array.from(new Set(قائمة)); }
function مدى(من, إلى) { const arr = []; if (من <= إلى) { for (let i = من; i <= إلى; i++) arr.push(i); } else { for (let i = من; i >= إلى; i--) arr.push(i); } return arr; }
function مجموع(قائمة) { return قائمة.reduce((a, b) => a + b, 0); }
function اختصر(قائمة) { return مجموع(قائمة); }
function متوسط(قائمة) { return قائمة.length ? مجموع(قائمة) / قائمة.length : 0; }
function أدنى(قائمة) { return Math.min.apply(null, قائمة); }
function أقصى(قائمة) { return Math.max.apply(null, قائمة); }
function عشوائي(من, إلى) { return Math.floor(Math.random() * (إلى - من + 1)) + من; }
function اقتطع(نص, من, عدد) { return String(نص).substr(من, عدد); }
function قسم(نص, فاصل) { return String(نص).split(فاصل); }
function انضم(قائمة, فاصل) { return قائمة.join(فاصل); }
function بحث(نص, جزء) { return String(نص).indexOf(جزء); }
function مكرر(نص, عدد) { return String(نص).repeat(عدد); }
function ازل_فراغات(نص) { return String(نص).trim(); }
function يبدأ_بـ(نص, بداية) { return String(نص).startsWith(بداية); }
function ينتهي_بـ(نص, نهاية) { return String(نص).endsWith(نهاية); }
function كبير(نص) { return String(نص).toUpperCase(); }
function صغير(نص) { return String(نص).toLowerCase(); }
function جذر(ع) { return Math.sqrt(ع); }
function قوة(أ, ب) { return Math.pow(أ, ب); }
function قوس(ع) { return Math.round(ع); }
function تقريب(ع) { return Math.round(ع); }
function أرضي(ع) { return Math.floor(ع); }
function سقف(ع) { return Math.ceil(ع); }
function مطلق(ع) { return Math.abs(ع); }
function أصغر(أ, ب) { return Math.min(أ, ب); }
function أكبر(أ, ب) { return Math.max(أ, ب); }
function مثلث(ع) { return Math.sin(ع); }
function جا(ع) { return Math.sin(ع); }
function جيب(ع) { return Math.cos(ع); }
function جتا(ع) { return Math.cos(ع); }
function ظل(ع) { return Math.tan(ع); }
function زمن() { return Math.floor(Date.now() / 1000); }
function سنة() { return new Date().getFullYear(); }
function تاريخ() { return new Date().toLocaleDateString("ar"); }
function طول(v) {
  if (typeof v === "string") return Array.from(v).length;
  if (Array.isArray(v)) return v.length;
  if (v && typeof v === "object") return Object.keys(v).length;
  return 0;
}
function اطبع() { console.log.apply(console, arguments); }


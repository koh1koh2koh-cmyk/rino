// ============================================
// Rino Playground Server
// ============================================

const http = require('http');
const fs = require('fs');
const path = require('path');
const { execFile } = require('child_process');

// ============================================
// الإعدادات
// ============================================
const PORT = 3000;
const RINO_PATH = 'C:\\Users\\user\\Desktop\\rino\\target\\release\\rino.exe';
const RINO_DIR = 'C:\\Users\\user\\Desktop\\rino';
const PUBLIC_DIR = path.join(__dirname, 'public');

// ============================================
// القالب الافتراضي
// ============================================
const DEFAULT_CODE = `صفحة("مرحبا Rino")

نمط {
  .عنوان_رئيسي {
    محاذاة: "وسط"
    لون: "أبيض"
    خلفية: "بنفسجي"
    حشوة: "30"
    استدارة: "15"
  }
}

حالة {
  دع عدّاد = 0
}

رأس_جميل("🎨 مرحبا بك في Rino")

عنوان("مثال تفاعلي", "", "عنوان_رئيسي")

فقرة("اضغط الزر وشاهد العداد يتغير:")

فقرة("العدد: " + عدّاد)

زر("زد +1", "", "rino-btn") عند_الضغط {
  عدّاد = عدّاد + 1
}

بطاقة("منتج مميز", "هذا منتج رائع", "100 جنيه")

تنبيه_نجاح("كل شيء يعمل!")
`;

// ============================================
// دالة تشغيل Rino
// ============================================
function compileRino(code) {
  return new Promise((resolve) => {
    // اكتب كود المستخدم في ملف مؤقت داخل مجلد rino
    const userFile = path.join(RINO_DIR, '__playground_input.rino');
    
    try {
      fs.writeFileSync(userFile, code, 'utf8');
    } catch (err) {
      return resolve({ ok: false, error: 'فشل حفظ الملف: ' + err.message });
    }

    // شغّل rino
    execFile(RINO_PATH, ['__playground_input.rino'], { cwd: RINO_DIR, timeout: 10000 }, (err, stdout, stderr) => {
      // احذف الملف المؤقت
      try { fs.unlinkSync(userFile); } catch (e) {}

      if (err) {
        let errorMsg = stderr || stdout || err.message;
        // احذف مسارات الملفات الطويلة
        errorMsg = errorMsg.replace(/C:\\[^\s]*/g, '').trim();
        return resolve({ ok: false, error: errorMsg || 'خطأ غير معروف' });
      }

      // اقرأ الملفات الثلاثة
      try {
        const htmlPath = path.join(RINO_DIR, '__playground_input.html');
        const cssPath = path.join(RINO_DIR, '__playground_input.css');
        const jsPath = path.join(RINO_DIR, '__playground_input.js');

        let html = fs.existsSync(htmlPath) ? fs.readFileSync(htmlPath, 'utf8') : '';
        const css = fs.existsSync(cssPath) ? fs.readFileSync(cssPath, 'utf8') : '';
        const js = fs.existsSync(jsPath) ? fs.readFileSync(jsPath, 'utf8') : '';

        // احذف الملفات المؤقتة
        try { fs.unlinkSync(htmlPath); } catch (e) {}
        try { fs.unlinkSync(cssPath); } catch (e) {}
        try { fs.unlinkSync(jsPath); } catch (e) {}

        // دمج CSS و JS في HTML
        html = html.replace(
          '<link rel="stylesheet" href="__playground_input.css">',
          '<style>' + css + '</style>'
        );
        html = html.replace(
          '<script src="__playground_input.js"></script>',
          '<script>' + js + '</script>'
        );

        resolve({ ok: true, html: html, raw: stdout });
      } catch (readErr) {
        resolve({ ok: false, error: 'فشل قراءة الملف الناتج: ' + readErr.message });
      }
    });
  });
}

// ============================================
// الخادم
// ============================================
const server = http.createServer(async (req, res) => {
  // CORS
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

  if (req.method === 'OPTIONS') {
    res.writeHead(200);
    return res.end();
  }

  // الصفحة الرئيسية
  if (req.method === 'GET' && (req.url === '/' || req.url === '/index.html')) {
    const indexPath = path.join(PUBLIC_DIR, 'index.html');
    try {
      const html = fs.readFileSync(indexPath, 'utf8');
      res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      return res.end(html);
    } catch (e) {
      res.writeHead(500);
      return res.end('ملف index.html غير موجود');
    }
  }

  // نقطة الترجمة
  if (req.method === 'POST' && req.url === '/compile') {
    let body = '';
    req.on('data', chunk => { body += chunk; });
    req.on('end', async () => {
      try {
        const data = JSON.parse(body);
        const code = data.code || '';
        
        if (!code.trim()) {
          res.writeHead(200, { 'Content-Type': 'application/json; charset=utf-8' });
          return res.end(JSON.stringify({ ok: false, error: 'الكود فارغ' }));
        }

        console.log('📝 ترجمة كود بطول', code.length, 'حرف...');
        const result = await compileRino(code);
        
        res.writeHead(200, { 'Content-Type': 'application/json; charset=utf-8' });
        return res.end(JSON.stringify(result));
      } catch (e) {
        res.writeHead(500, { 'Content-Type': 'application/json; charset=utf-8' });
        return res.end(JSON.stringify({ ok: false, error: 'خطأ في الخادم: ' + e.message }));
      }
    });
    return;
  }

  // كود افتراضي
  if (req.method === 'GET' && req.url === '/default') {
    res.writeHead(200, { 'Content-Type': 'application/json; charset=utf-8' });
    return res.end(JSON.stringify({ code: DEFAULT_CODE }));
  }

  res.writeHead(404);
  res.end('غير موجود');
});

server.listen(PORT, () => {
  console.log('');
  console.log('🎨 ================================');
  console.log('🎨  Rino Playground');
  console.log('🎨 ================================');
  console.log('');
  console.log('🌐 افتح المتصفح على:');
  console.log('   http://localhost:' + PORT);
  console.log('');
  console.log('⏹️  لإيقاف: Ctrl+C');
  console.log('');
});
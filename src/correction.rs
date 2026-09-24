//! أدوات التصحيح التلقائي للأخطاء الإملائية.

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..=n { dp[i][0] = i; }
    for j in 0..=m { dp[0][j] = j; }
    for i in 1..=n {
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[n][m]
}

const KEYWORDS: &[&str] = &[
    "صفحة", "نمط", "حالة", "جسم", "منطق",
    "دع", "ثابت", "دالة", "أرجع",
    "إذا", "وإلا", "لكل", "في",
    "من", "إلى", "خطوة",
    "حاول", "أمسك",
    "و", "أو", "ليس", "صحيح", "خطأ", "لا_شيء",
    "إعدادات", "تصحيح", "مكون",
    "عند_المرور",
    "عنوان", "فقرة", "زر", "صورة", "رابط", "مدخل",
    "قائمة", "عنصر_قائمة",
    "قسم", "منطقة", "ترويسة", "تذييل",
    "عريض", "مائل", "فاصل",
    "جدول", "صف", "خلية", "فيديو", "موسيقى", "فاصل_أفقي",
    "اطبع",
    "عند_الضغط", "عند_التغيير",
];

/// كلمات محمية — لا تُصحَّح أبدًا (حتى لو كانت قريبة من كلمات أخرى)
const PROTECTED: &[&str] = &[
    "عرض",   // عرض ≠ عريض
    "من",    // من ≠ في
    "إلى",
    "صف",    // صف ≠ ضعف
    "جدول",
    "خلية",
];

pub fn suggest_keyword(input: &str) -> Option<&'static str> {
    if PROTECTED.contains(&input) {
        return None;
    }

    let mut best: Option<(&'static str, usize)> = None;
    for &kw in KEYWORDS {
        let dist = levenshtein(input, kw);
        if dist <= 1 {
            match best {
                Some((_, d)) if d <= dist => {}
                _ => best = Some((kw, dist)),
            }
        }
    }
    best.map(|(k, _)| k)
}
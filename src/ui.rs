#![allow(clippy::too_many_arguments)]
//! Tela de informacoes (design 960x540, mesma linguagem visual do rg52-led): seis
//! cartoes com barras de uso. Desenhada por software em um `Pixmap`; o `main` so a envia
//! para uma textura SDL, entao a tela tambem e testada e fotografada sem janela.

use crate::gfx::{Color, Face, Fonts, Pixmap};
use crate::sysinfo::{fmt_bytes, fmt_kb, fmt_uptime, Info};

pub const DESIGN_W: f32 = 960.0;
pub const DESIGN_H: f32 = 540.0;

const BG: Color = Color::hex(0x0B0D12);
const TEXT: Color = Color::hex(0xE8ECF4);
const MUTED: Color = Color::hex(0x8E98AD);
const LINE: Color = Color::hex(0x1D2332);
const CARD: Color = Color::hex(0x12161F);
const TRACK: Color = Color::hex(0x242B3B);
const KEYCAP_BG: Color = Color::hex(0x1B2130);
const KEYCAP_BORDER: Color = Color::hex(0x2A3144);
const OK: Color = Color::hex(0x4ADE80);
const WARN: Color = Color::hex(0xFFB938);
const BAD: Color = Color::hex(0xFF6B6B);
pub const ACCENT: Color = Color::hex(0xFFB938);

pub fn parse_accent(s: &str) -> Result<Color, String> {
    match s.to_ascii_lowercase().as_str() {
        "amber" => Ok(ACCENT),
        "teal" => Ok(Color::hex(0x5EEAD4)),
        "violet" => Ok(Color::hex(0xA78BFA)),
        "coral" => Ok(Color::hex(0xFF7A59)),
        t => {
            let h = t.strip_prefix('#').unwrap_or(t);
            match (h.len(), u32::from_str_radix(h, 16)) {
                (6, Ok(v)) => Ok(Color::hex(v)),
                _ => Err(format!(
                    "cor invalida: {s:?} (use amber, teal, violet, coral ou #RRGGBB)"
                )),
            }
        }
    }
}

/// Verde ate 60%, ambar ate 85%, vermelho acima: o uso de memoria, disco e carga.
pub fn level_color(pct: f32) -> Color {
    if pct < 60.0 {
        OK
    } else if pct < 85.0 {
        WARN
    } else {
        BAD
    }
}

/// Bateria: vermelho abaixo de 15%, ambar abaixo de 30%.
pub fn battery_color(pct: u8) -> Color {
    if pct < 15 {
        BAD
    } else if pct < 30 {
        WARN
    } else {
        OK
    }
}

struct P<'a> {
    pm: &'a mut Pixmap,
    f: &'a Fonts,
    k: f32,
    ox: f32,
    oy: f32,
}

#[derive(Clone, Copy)]
struct R {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// Logo (RGBA, 264x56 = 2x do tamanho no design) e o tamanho em que e desenhado.
const LOGO: &[u8] = include_bytes!("../assets/logo_264x56.rgba");
const LOGO_SRC: (usize, usize) = (264, 56);
const LOGO_W: f32 = 132.0;
const LOGO_H: f32 = 27.8;
const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

impl P<'_> {
    /// Desenha o logo centrado em (`cx`, `cy`) (coordenadas do design). A versao
    /// reamostrada fica em cache, ja que o desenho pode rodar uma vez por segundo.
    fn logo(&mut self, cx: f32, cy: f32) {
        thread_local! {
            static CACHE: std::cell::RefCell<Option<(usize, usize, Pixmap)>> =
                const { std::cell::RefCell::new(None) };
        }
        let (dw, dh) = (
            (LOGO_W * self.k).round() as usize,
            (LOGO_H * self.k).round() as usize,
        );
        let (x, y) = (
            (self.ox + (cx - LOGO_W / 2.0) * self.k).round() as i32,
            (self.oy + (cy - LOGO_H / 2.0) * self.k).round() as i32,
        );
        CACHE.with(|c| {
            let mut c = c.borrow_mut();
            if !matches!(&*c, Some((w, h, _)) if *w == dw && *h == dh) {
                let pm = Pixmap::from_rgba_scaled(LOGO, LOGO_SRC.0, LOGO_SRC.1, dw, dh);
                *c = Some((dw, dh, pm));
            }
            if let Some((_, _, pm)) = &*c {
                self.pm.blit(pm, x, y, 1.0);
            }
        });
    }
    fn rrect(&mut self, r: R, rad: f32, c: Color) {
        let k = self.k;
        self.pm.fill_rrect(
            self.ox + r.x * k,
            self.oy + r.y * k,
            r.w * k,
            r.h * k,
            rad * k,
            c,
        );
    }
    fn stroke(&mut self, r: R, rad: f32, bw: f32, c: Color) {
        let k = self.k;
        self.pm.stroke_rrect(
            self.ox + r.x * k,
            self.oy + r.y * k,
            r.w * k,
            r.h * k,
            rad * k,
            bw * k,
            c,
        );
    }
    fn circle(&mut self, cx: f32, cy: f32, r: f32, c: Color) {
        let k = self.k;
        self.pm
            .fill_circle(self.ox + cx * k, self.oy + cy * k, r * k, c);
    }
    fn ring(&mut self, cx: f32, cy: f32, r: f32, bw: f32, c: Color) {
        let k = self.k;
        self.pm
            .stroke_circle(self.ox + cx * k, self.oy + cy * k, r * k, bw * k, c);
    }
    fn width(&self, face: Face, size: f32, sp: f32, s: &str) -> f32 {
        self.f.width(face, size * self.k, sp * self.k, s) / self.k
    }
    fn text(
        &mut self,
        face: Face,
        size: f32,
        sp: f32,
        c: Color,
        x: f32,
        baseline: f32,
        s: &str,
    ) -> f32 {
        let k = self.k;
        self.pm.text(
            self.f,
            face,
            size * k,
            sp * k,
            c,
            self.ox + x * k,
            self.oy + baseline * k,
            s,
        ) / k
    }
    /// Texto centrado verticalmente em `cy` (como o `align-items:center` do CSS).
    fn text_mid(
        &mut self,
        face: Face,
        size: f32,
        sp: f32,
        c: Color,
        x: f32,
        cy: f32,
        s: &str,
    ) -> f32 {
        let (a, d) = self.f.metrics(face);
        self.text(face, size, sp, c, x, cy + (a + d) * size / 2.0, s)
    }
    /// Texto alinhado a direita em `right`, cortado com reticencias se passar de `max_w`.
    fn text_right(
        &mut self,
        face: Face,
        size: f32,
        c: Color,
        right: f32,
        cy: f32,
        s: &str,
        max_w: f32,
    ) {
        let s = self.f.fit(face, size * self.k, 0.0, s, max_w * self.k);
        let w = self.width(face, size, 0.0, &s);
        self.text_mid(face, size, 0.0, c, right - w, cy, &s);
    }
}

const PAD: f32 = 18.0;
const CARD_W: f32 = 288.0;
const CARD_H: f32 = 196.0;
const GAP: f32 = 16.0;
const X0: f32 = 32.0;
const Y0: f32 = 80.0;
const ROW: f32 = 26.0;

fn card_rect(i: usize) -> R {
    R {
        x: X0 + (i % 3) as f32 * (CARD_W + GAP),
        y: Y0 + (i / 3) as f32 * (CARD_H + GAP),
        w: CARD_W,
        h: CARD_H,
    }
}

fn card(p: &mut P, i: usize, title: &str, accent: Color) -> R {
    let r = card_rect(i);
    p.rrect(r, 18.0, CARD);
    p.stroke(r, 18.0, 1.0, LINE);
    let cy = r.y + PAD + 6.0;
    p.circle(r.x + PAD + 3.5, cy, 3.5, accent);
    p.text_mid(
        Face::Sans800,
        12.0,
        1.92,
        MUTED,
        r.x + PAD + 14.0,
        cy,
        title,
    );
    r
}

/// Linha "rotulo ........ valor".
fn kv(p: &mut P, r: R, n: usize, key: &str, val: &str, col: Color) {
    let cy = r.y + 50.0 + n as f32 * ROW;
    p.text_mid(Face::Sans600, 13.0, 0.0, MUTED, r.x + PAD, cy, key);
    let kw = p.width(Face::Sans600, 13.0, 0.0, key);
    p.text_right(
        Face::Mono700,
        13.0,
        col,
        r.x + r.w - PAD,
        cy,
        val,
        r.w - 2.0 * PAD - kw - 12.0,
    );
}

/// Barra de uso com rotulo e valor acima, `y` = topo do rotulo.
fn bar(p: &mut P, r: R, y: f32, label: &str, value: &str, pct: f32, col: Color) {
    p.text_mid(Face::Sans600, 13.0, 0.0, MUTED, r.x + PAD, y + 8.0, label);
    p.text_right(
        Face::Mono700,
        12.0,
        TEXT,
        r.x + r.w - PAD,
        y + 8.0,
        value,
        r.w - 2.0 * PAD - 70.0,
    );
    let t = R {
        x: r.x + PAD,
        y: y + 20.0,
        w: r.w - 2.0 * PAD,
        h: 8.0,
    };
    p.rrect(t, 4.0, TRACK);
    let w = (t.w * pct.clamp(0.0, 100.0) / 100.0).max(if pct > 0.0 { 8.0 } else { 0.0 });
    if w > 0.0 {
        p.rrect(R { w, ..t }, 4.0, col);
    }
}

/// "47°C" (a fonte embutida tem o glifo do grau).
fn temp(t: Option<f32>) -> String {
    t.map_or("-".into(), |v| format!("{v:.0}\u{00B0}C"))
}

/// Desenha o quadro completo em `pm` (tamanho livre; o design e centralizado e escalado).
pub fn render(
    pm: &mut Pixmap,
    fonts: &Fonts,
    info: &Info,
    clock: &(String, String),
    accent: Color,
) {
    let (w, h) = (pm.width() as f32, pm.height() as f32);
    let k = (w / DESIGN_W).min(h / DESIGN_H);
    let (ox, oy) = ((w - DESIGN_W * k) / 2.0, (h - DESIGN_H * k) / 2.0);
    pm.clear(BG);
    let mut p = P {
        pm,
        f: fonts,
        k,
        ox,
        oy,
    };

    // --- cabecalho ---
    let cy = 44.0;
    p.ring(41.0, cy, 6.5, 2.0, TEXT);
    p.ring(53.0, cy, 6.5, 2.0, accent);
    let w1 = p.text_mid(Face::Sans800, 24.0, -0.48, TEXT, 74.0, cy, "RG52");
    p.text_mid(
        Face::Sans500,
        24.0,
        -0.48,
        MUTED,
        74.0 + w1 + 6.0,
        cy,
        "INFO",
    );
    p.logo(480.0, cy);
    let tw = p.width(Face::Mono700, 22.0, 0.0, &clock.1);
    p.text_mid(
        Face::Mono700,
        22.0,
        0.0,
        TEXT,
        928.0 - tw,
        cy - 7.0,
        &clock.1,
    );
    let dw = p.width(Face::Sans600, 12.0, 0.0, &clock.0);
    p.text_mid(
        Face::Sans600,
        12.0,
        0.0,
        MUTED,
        928.0 - dw,
        cy + 14.0,
        &clock.0,
    );

    // --- 0: sistema ---
    let r = card(&mut p, 0, "SYSTEM", accent);
    kv(&mut p, r, 0, "Host", &info.host, TEXT);
    let users = if info.users.is_empty() {
        "0".to_string()
    } else {
        format!("{} ({})", info.users.len(), info.users.join(", "))
    };
    kv(&mut p, r, 1, "Users", &users, TEXT);
    kv(&mut p, r, 2, "Uptime", &fmt_uptime(info.uptime_s), TEXT);
    let l = info.load;
    kv(
        &mut p,
        r,
        3,
        "Load",
        &format!("{:.2} {:.2} {:.2}", l[0], l[1], l[2]),
        TEXT,
    );
    let lp = info.load_pct();
    bar(
        &mut p,
        r,
        r.y + 50.0 + 4.0 * ROW - 2.0,
        "Load / cores",
        &format!("{lp:.0}%"),
        lp,
        level_color(lp),
    );

    // --- 1: cpu ---
    let r = card(&mut p, 1, "CPU", accent);
    let big = temp(info.cpu_temp);
    let col = info
        .cpu_temp
        .map_or(MUTED, |t| level_color((t - 20.0) * 100.0 / 60.0));
    p.text_mid(Face::Mono700, 34.0, 0.0, col, r.x + PAD, r.y + 68.0, &big);
    p.text_right(
        Face::Sans600,
        12.0,
        MUTED,
        r.x + r.w - PAD,
        r.y + 68.0,
        &format!("{} cores", info.cores),
        120.0,
    );
    let model = if info.cpu_model.is_empty() {
        "-"
    } else {
        &info.cpu_model
    };
    kv(&mut p, r, 2, "Model", model, TEXT);
    let ty = if info.cpu_type.is_empty() {
        "-"
    } else {
        &info.cpu_type
    };
    kv(&mut p, r, 3, "Sensor", ty, TEXT);
    let tp = info
        .cpu_temp
        .map_or(0.0, |t| ((t - 20.0) / 80.0 * 100.0).clamp(0.0, 100.0));
    bar(
        &mut p,
        r,
        r.y + 50.0 + 4.0 * ROW - 2.0,
        "Temperature",
        &temp(info.cpu_temp),
        tp,
        col,
    );

    // --- 2: bateria ---
    let r = card(&mut p, 2, "BATTERY", accent);
    match &info.battery {
        Some(b) => {
            let pctv = b.capacity.unwrap_or(0);
            let col = if b.capacity.is_some() {
                battery_color(pctv)
            } else {
                MUTED
            };
            let big = b.capacity.map_or("-".into(), |c| format!("{c}%"));
            p.text_mid(Face::Mono700, 34.0, 0.0, col, r.x + PAD, r.y + 68.0, &big);
            let st = if b.status.is_empty() { "-" } else { &b.status };
            p.text_right(
                Face::Sans700,
                13.0,
                MUTED,
                r.x + r.w - PAD,
                r.y + 68.0,
                st,
                130.0,
            );
            kv(&mut p, r, 2, "Temp", &temp(b.temp), TEXT);
            let v = b.voltage.map_or("-".into(), |v| format!("{v:.2} V"));
            kv(&mut p, r, 3, "Voltage", &v, TEXT);
            let a = b.current.map_or("-".into(), |v| format!("{v:.2} A"));
            kv(&mut p, r, 4, "Current", &a, TEXT);
            let t = R {
                x: r.x + PAD,
                y: r.y + r.h - PAD - 8.0,
                w: r.w - 2.0 * PAD,
                h: 8.0,
            };
            p.rrect(t, 4.0, TRACK);
            let w = (t.w * pctv as f32 / 100.0).max(if pctv > 0 { 8.0 } else { 0.0 });
            if w > 0.0 {
                p.rrect(R { w, ..t }, 4.0, col);
            }
        }
        None => {
            p.text_mid(
                Face::Sans600,
                14.0,
                0.0,
                MUTED,
                r.x + PAD,
                r.y + 70.0,
                "No battery found",
            );
        }
    }

    // --- 3: memoria ---
    let r = card(&mut p, 3, "MEMORY", accent);
    let (mp, sp) = (info.mem_pct(), info.swap_pct());
    p.text_mid(
        Face::Mono700,
        34.0,
        0.0,
        level_color(mp),
        r.x + PAD,
        r.y + 68.0,
        &format!("{mp:.0}%"),
    );
    p.text_right(
        Face::Sans600,
        12.0,
        MUTED,
        r.x + r.w - PAD,
        r.y + 68.0,
        &format!("of {}", fmt_kb(info.mem_total)),
        130.0,
    );
    bar(
        &mut p,
        r,
        r.y + 96.0,
        "RAM",
        &format!("{} / {}", fmt_kb(info.mem_used()), fmt_kb(info.mem_total)),
        mp,
        level_color(mp),
    );
    bar(
        &mut p,
        r,
        r.y + 140.0,
        "Swap",
        &format!("{} / {}", fmt_kb(info.swap_used()), fmt_kb(info.swap_total)),
        sp,
        level_color(sp),
    );

    // --- 4: rede ---
    let r = card(&mut p, 4, "NETWORK", accent);
    match &info.net {
        Some((iface, ip)) => {
            p.text_mid(Face::Mono700, 26.0, 0.0, TEXT, r.x + PAD, r.y + 76.0, ip);
            p.circle(r.x + PAD + 4.0, r.y + 112.0, 4.0, OK);
            p.text_mid(
                Face::Sans600,
                13.0,
                0.0,
                MUTED,
                r.x + PAD + 16.0,
                r.y + 112.0,
                &format!("Connected on {iface}"),
            );
        }
        None => {
            p.text_mid(
                Face::Mono700,
                26.0,
                0.0,
                MUTED,
                r.x + PAD,
                r.y + 76.0,
                "No address",
            );
            p.circle(r.x + PAD + 4.0, r.y + 112.0, 4.0, BAD);
            p.text_mid(
                Face::Sans600,
                13.0,
                0.0,
                MUTED,
                r.x + PAD + 16.0,
                r.y + 112.0,
                "Offline",
            );
        }
    }

    // --- 5: armazenamento ---
    let r = card(&mut p, 5, "STORAGE", accent);
    for (i, d) in info.disks.iter().enumerate() {
        let y = r.y + 44.0 + i as f32 * 46.0;
        let mounted = d.fs.is_some() && d.total > 0;
        let label = d.path.as_str();
        let val = if mounted {
            format!("{} free of {}", fmt_bytes(d.free()), fmt_bytes(d.total))
        } else {
            "not mounted".to_string()
        };
        let pctv = if mounted { d.pct() } else { 0.0 };
        bar(&mut p, r, y, label, &val, pctv, level_color(pctv));
    }

    // --- rodape ---
    let cy = 514.0;
    let (kw, key) = (p.width(Face::Mono700, 11.0, 0.0, "B") + 12.0, "B");
    let kw = kw.max(24.0);
    let kr = R {
        x: X0,
        y: cy - 12.0,
        w: kw,
        h: 24.0,
    };
    p.rrect(kr, 7.0, KEYCAP_BG);
    p.stroke(kr, 7.0, 1.0, KEYCAP_BORDER);
    let tw = p.width(Face::Mono700, 11.0, 0.0, key);
    p.text_mid(
        Face::Mono700,
        11.0,
        0.0,
        TEXT,
        X0 + (kw - tw) / 2.0,
        cy,
        key,
    );
    p.text_mid(Face::Sans600, 13.0, 0.0, MUTED, X0 + kw + 8.0, cy, "Exit");
    let vw = p.width(Face::Mono500, 12.0, 0.0, VERSION);
    p.text_mid(
        Face::Mono500,
        12.0,
        0.0,
        MUTED,
        480.0 - vw / 2.0,
        cy,
        VERSION,
    );
    let live = "Updates every second";
    let lw = p.width(Face::Sans600, 12.0, 0.0, live);
    p.text_mid(Face::Sans600, 12.0, 0.0, MUTED, 928.0 - lw, cy, live);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sysinfo::{Battery, Disk};

    fn info() -> Info {
        Info {
            host: "rg52".into(),
            users: vec!["ark".into()],
            uptime_s: 3700,
            load: [0.5, 0.5, 0.5],
            cores: 4,
            cpu_model: "RK3562".into(),
            cpu_type: "soc-thermal".into(),
            cpu_temp: Some(47.0),
            battery: Some(Battery {
                capacity: Some(83),
                status: "Discharging".into(),
                voltage: Some(3.98),
                current: Some(-0.45),
                temp: Some(31.0),
            }),
            mem_total: 2_000_000,
            mem_avail: 500_000,
            swap_total: 1_000_000,
            swap_free: 1_000_000,
            net: Some(("wlan0".into(), "192.168.0.23".into())),
            disks: vec![
                Disk {
                    path: "/".into(),
                    fs: Some("/dev/root".into()),
                    total: 64 << 30,
                    used: 10 << 30,
                },
                Disk {
                    path: "/roms".into(),
                    fs: Some("/dev/sda1".into()),
                    total: 128 << 30,
                    used: 120 << 30,
                },
                Disk {
                    path: "/roms2".into(),
                    fs: None,
                    total: 0,
                    used: 0,
                },
            ],
        }
    }

    fn clock() -> (String, String) {
        ("Tue 03 Nov 2026".into(), "21:29:05".into())
    }

    fn shot(w: usize, h: usize, i: &Info) -> Pixmap {
        let mut pm = Pixmap::new(w, h);
        render(&mut pm, &Fonts::new(), i, &clock(), ACCENT);
        pm
    }

    fn near(a: [u8; 4], b: Color, tol: i32) -> bool {
        (a[0] as i32 - b.r as i32).abs() <= tol
            && (a[1] as i32 - b.g as i32).abs() <= tol
            && (a[2] as i32 - b.b as i32).abs() <= tol
    }

    #[test]
    fn cores_por_nivel() {
        assert_eq!(level_color(10.0), OK);
        assert_eq!(level_color(70.0), WARN);
        assert_eq!(level_color(95.0), BAD);
        assert_eq!(battery_color(10), BAD);
        assert_eq!(battery_color(20), WARN);
        assert_eq!(battery_color(80), OK);
        assert_eq!(parse_accent("teal").unwrap(), Color::hex(0x5EEAD4));
        assert_eq!(parse_accent("#a78bfa").unwrap(), Color::hex(0xA78BFA));
        assert!(parse_accent("roxo").is_err());
    }

    #[test]
    fn cartoes_nao_se_sobrepoem_e_cabem_na_tela() {
        for i in 0..6 {
            let a = card_rect(i);
            assert!(a.x >= X0 && a.x + a.w <= 928.0 + 0.01, "{i}");
            assert!(a.y + a.h <= 540.0 - 36.0, "{i} invade o rodape");
            for j in i + 1..6 {
                let b = card_rect(j);
                let ov = a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
                assert!(!ov, "{i} x {j}");
            }
        }
    }

    #[test]
    fn desenha_cartoes_e_barras() {
        let pm = shot(960, 540, &info());
        assert!(near(pm.get(2, 2).unwrap(), BG, 1));
        // fundo do cartao de sistema
        let c = card_rect(0);
        assert!(near(
            pm.get((c.x + c.w - 20.0) as usize, (c.y + 20.0) as usize)
                .unwrap(),
            CARD,
            3
        ));
        // barra de RAM (75% => verde ate 60, ambar ate 85): ambar na ponta da parte cheia
        let m = card_rect(3);
        let ram_y = (m.y + 96.0 + 24.0) as usize;
        let filled = pm.get((m.x + PAD + 10.0) as usize, ram_y).unwrap();
        assert!(near(filled, WARN, 6), "{filled:?}");
        let empty = pm.get((m.x + m.w - PAD - 10.0) as usize, ram_y).unwrap();
        assert!(near(empty, TRACK, 4), "{empty:?}");
        // disco quase cheio (120/128 = 94%): barra vermelha
        let s = card_rect(5);
        let y = (s.y + 44.0 + 46.0 + 24.0) as usize;
        assert!(near(
            pm.get((s.x + PAD + 10.0) as usize, y).unwrap(),
            BAD,
            6
        ));
        // /roms2 nao montado: so o trilho
        let y = (s.y + 44.0 + 92.0 + 24.0) as usize;
        assert!(near(
            pm.get((s.x + PAD + 10.0) as usize, y).unwrap(),
            TRACK,
            4
        ));
    }

    #[test]
    fn bateria_ausente_e_sem_rede_nao_quebram() {
        let mut i = info();
        i.battery = None;
        i.net = None;
        i.cpu_temp = None;
        i.disks.clear();
        let a = shot(960, 540, &i);
        let b = shot(960, 540, &info());
        assert_ne!(a.data(), b.data());
        // valores vazios tambem
        shot(960, 540, &Info::default());
    }

    #[test]
    fn escala_e_letterbox() {
        let pm = shot(1024, 768, &info());
        assert!(near(pm.get(1, 1).unwrap(), BG, 1));
        assert!(near(pm.get(1022, 766).unwrap(), BG, 1));
        // 1280x720 e 1x1 tambem renderizam
        shot(1280, 720, &info());
        shot(1, 1, &info());
    }

    #[test]
    fn texto_longo_e_cortado_dentro_do_cartao() {
        let mut i = info();
        i.cpu_model = "Um nome de processador absurdamente longo que nao cabe no cartao".into();
        i.users = (0..30).map(|n| format!("user{n}")).collect();
        let pm = shot(960, 540, &i);
        // 2 px a direita do cartao de CPU (no vao entre cartoes) continua fundo
        let c = card_rect(1);
        for y in [c.y + 76.0, c.y + 102.0, c.y + 128.0] {
            assert!(near(
                pm.get((c.x + c.w + 6.0) as usize, y as usize).unwrap(),
                BG,
                3
            ));
        }
        let c = card_rect(0);
        assert!(near(
            pm.get((c.x + c.w + 6.0) as usize, (c.y + 76.0) as usize)
                .unwrap(),
            BG,
            3
        ));
    }

    #[test]
    fn captura_para_inspecao() {
        if let Ok(p) = std::env::var("RG52_INFO_DUMP") {
            shot(1280, 720, &info())
                .write_ppm(std::path::Path::new(&p))
                .unwrap();
        }
    }
}

//! Audit automatique du simulateur : joue N matchs sans affichage et contrôle à chaque instant
//! l'intégrité de la grille, les règles (passes en avant, exclus) et le réalisme des statistiques.
//! Ouvre index.html directement (file://, sans serveur) dans Chromium, piloté par le protocole DevTools.
//! Outil de développement : ni dans les archives publiées, ni dans bin/.
//!
//! Utilisation : cargo run -p audit -- 40      (CHROME=/chemin/vers/chrome pour un navigateur existant)
//! Sort en erreur s'il y a une violation ou une erreur dans la page.
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use headless_chrome::protocol::cdp::types::Event;
use headless_chrome::{Browser, LaunchOptions, Tab};
use serde_json::Value;

/// Le JavaScript injecté, inchangé depuis l'ancien banc Python (%d : le nombre de matchs).
const JS: &str = include_str!("audit.js");

/// Chromium de Playwright, s'il est là : /opt/pw-browsers/chromium, ou chromium-<révision>/chrome-linux*/chrome.
fn chromium_playwright() -> Option<PathBuf> {
    let base = Path::new("/opt/pw-browsers");
    if base.join("chromium").is_file() {
        return Some(base.join("chromium"));
    }
    let mut dossiers: Vec<PathBuf> = std::fs::read_dir(base)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("chromium") && !n.contains("headless")))
        .collect();
    dossiers.sort();
    dossiers
        .iter()
        .rev()
        .flat_map(|d| [d.join("chrome"), d.join("chrome-linux/chrome"), d.join("chrome-linux64/chrome")])
        .find(|p| p.is_file())
}

/// Attend la page chargée. Pas wait_until_navigated : une page locale se charge si vite que
/// l'événement qu'il attend passe avant lui, et il attend alors pour rien.
fn attendre_chargement(tab: &Tab) -> Result<()> {
    for _ in 0..300 {
        let r = tab.evaluate("location.href !== 'about:blank' && document.readyState === 'complete'", false);
        if r.ok().and_then(|o| o.value) == Some(Value::Bool(true)) {
            return Ok(());
        }
        sleep(Duration::from_millis(100));
    }
    bail!("la page ne s'est pas chargée en 30 s")
}

fn main() -> Result<()> {
    let n: u32 = match std::env::args().nth(1) {
        Some(a) => a.parse::<u32>().with_context(|| format!("nombre de matchs attendu, reçu « {a} »"))?,
        None => 40,
    };
    // depuis la racine du dépôt (cargo run), sinon à côté des sources de la caisse
    let a_cote = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../index.html");
    let page = [Path::new("index.html"), a_cote.as_path()]
        .into_iter()
        .find(|p| p.is_file())
        .context("index.html introuvable : lancer depuis la racine du dépôt")?;
    let url = url::Url::from_file_path(std::path::absolute(page)?).map_err(|()| anyhow::anyhow!("chemin de index.html invalide"))?;

    // sans CHROME ni Chromium de Playwright : headless_chrome cherche chrome / chromium dans le PATH
    let browser = Browser::new(LaunchOptions {
        path: std::env::var_os("CHROME").map(PathBuf::from).or_else(chromium_playwright),
        sandbox: false, // --no-sandbox, comme l'ancien banc (conteneurs, CI)
        // les N matchs se jouent en un seul appel, sans aucun message du navigateur entre-temps
        idle_browser_timeout: Duration::from_secs(6 * 3600),
        ..LaunchOptions::default()
    })?;
    let tab = browser.wait_for_initial_tab()?;
    let erreurs = Arc::new(Mutex::new(Vec::<String>::new()));
    let e = Arc::clone(&erreurs);
    tab.add_event_listener(Arc::new(move |ev: &Event| {
        if let Event::RuntimeExceptionThrown(ev) = ev {
            let v = serde_json::to_value(&ev.params).unwrap_or_default();
            let d = &v["exceptionDetails"];
            let texte = d["exception"]["description"].as_str().or(d["text"].as_str()).unwrap_or("?");
            e.lock().unwrap().push(texte.lines().next().unwrap_or("").to_string());
        }
    }))?;
    tab.enable_runtime()?;
    tab.navigate_to(url.as_str())?;
    attendre_chargement(&tab)?;
    sleep(Duration::from_millis(500));

    // l'objet revient en texte JSON : evaluate ne renvoie la valeur que des types simples
    let r = tab.evaluate(&format!("JSON.stringify(({})())", JS.replace("%d", &n.to_string())), false)?;
    let Some(Value::String(texte)) = &r.value else {
        bail!("l'audit a échoué dans la page : {}", r.description.as_deref().unwrap_or("pas de résultat"));
    };
    let bilan: Value = serde_json::from_str(texte)?;
    println!("{}", serde_json::to_string_pretty(&bilan)?);
    let erreurs = erreurs.lock().unwrap().clone();
    println!("erreurs page: {erreurs:?}");

    let violations = bilan["violations"].as_object().map_or(0, |v| v.len());
    if violations > 0 || !erreurs.is_empty() {
        bail!("{violations} sorte(s) de violation, {} erreur(s) de page", erreurs.len());
    }
    println!("Bilan : {n} matchs, aucune violation, aucune erreur de page.");
    Ok(())
}

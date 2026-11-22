//! Serveur de Rugby Top 14 : sert index.html au réseau local, derrière la garde commune.
//!
//!     rugby [--port 8796] [--local] [--sans-navigateur] [--sans-pause] [--sans-maj]
//!
//! La saison s'enregistre dans le navigateur : le serveur n'a aucune route qui écrit.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use web_tools_commun::appairage::{self, Garde};
use web_tools_commun::serveur::{decoder_chemin, Reponse, Requete, Serveur};

pub const PORT: u16 = 8796;

/// La racine de l'outil (index.html) : le dossier courant s'il contient index.html, sinon le premier
/// dossier au-dessus de l'exécutable qui le contient.
pub fn racine() -> &'static Path {
    static R: OnceLock<PathBuf> = OnceLock::new();
    R.get_or_init(|| {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let page = |d: &Path| d.join("index.html").is_file();
        if page(&cwd) {
            return cwd;
        }
        let exe = std::env::current_exe().ok().and_then(|e| std::fs::canonicalize(e).ok());
        exe.and_then(|e| e.ancestors().find(|d| page(d)).map(Path::to_path_buf)).unwrap_or(cwd)
    })
}

static GARDE: OnceLock<Garde> = OnceLock::new();

fn garde() -> &'static Garde {
    GARDE.get().expect("garde non initialisée")
}

fn servir(r: &mut Requete) -> Reponse {
    // Host, Internet, TLS, jeton : avant tout traitement de la requête
    if let Some(rep) = garde().filtrer(r) {
        return rep;
    }
    if !r.lecture() {
        return Reponse::texte(501, "méthode non prise en charge");
    }
    // la page seule : ni .git/, ni les sources, ni les bancs
    match decoder_chemin(r.chemin()).as_str() {
        "/" | "/index.html" => match std::fs::read(racine().join("index.html")) {
            Ok(o) => Reponse::new(200, "text/html; charset=utf-8", o),
            Err(_) => Reponse::texte(404, "introuvable"),
        },
        _ => Reponse::texte(404, "introuvable"),
    }
}

/// En-têtes posés sur toutes les réponses (X-Frame-Options est déjà posé par la caisse commune).
fn repondre(r: &mut Requete) -> Reponse {
    let mut rep = servir(r);
    rep.entetes.retain(|(k, _)| !k.eq_ignore_ascii_case("Cache-Control"));
    rep.entete("Cache-Control", "no-store, max-age=0")
        .entete("Content-Security-Policy", "frame-ancestors 'self'")
        .entete("X-Content-Type-Options", "nosniff")
}

// ---------------------------------------------------------------------- lancement

#[derive(Debug, Default, PartialEq)]
pub struct Args {
    pub port: u16,
    pub local: bool,
    pub sans_navigateur: bool,
    pub sans_pause: bool,
    pub sans_maj: bool,
    pub aide: bool,
}

pub const USAGE: &str = "usage: rugby [--port 8796] [--local] [--sans-navigateur] [--sans-pause] [--sans-maj]";

pub fn lire_args(argv: &[String]) -> Result<Args, String> {
    let mut a = Args { port: PORT, ..Default::default() };
    let mut i = 0;
    while i < argv.len() {
        let x = argv[i].as_str();
        let (cle, val) = match x.split_once('=') {
            Some((k, v)) => (k, Some(v.to_string())),
            None => (x, None),
        };
        match cle {
            "--port" => {
                let v = match val {
                    Some(v) => v,
                    None => {
                        i += 1;
                        argv.get(i).cloned().ok_or("--port attend un nombre")?
                    }
                };
                a.port = v.parse().map_err(|_| format!("port invalide : {v}"))?;
            }
            "--local" if val.is_none() => a.local = true,
            "--sans-navigateur" if val.is_none() => a.sans_navigateur = true,
            "--sans-pause" if val.is_none() => a.sans_pause = true,
            "--sans-maj" if val.is_none() => a.sans_maj = true,
            "-h" | "--help" if val.is_none() => a.aide = true,
            _ => return Err(format!("option inconnue : {x}")),
        }
        i += 1;
    }
    Ok(a)
}

fn ouvrir_navigateur(url: &str) {
    let r = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", "", url]).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else {
        Command::new("xdg-open").arg(url).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
    };
    if r.is_err() {
        println!("Ouvrez {url} dans votre navigateur.");
    }
}

/// git dans le dossier de l'outil, avec un délai (s) : jamais d'attente sans fin du réseau.
/// Rend la sortie, ou l'erreur de git.
fn git(args: &[&str], delai: u64) -> Result<String, String> {
    // shortcut: sorties lues après la fin de git ; ces commandes n'écrivent que quelques lignes (pas de tampon plein)
    let mut proc = Command::new("git")
        .args(args)
        .current_dir(racine())
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let fin = Instant::now() + Duration::from_secs(delai);
    loop {
        match proc.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < fin => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = proc.kill();
                let _ = proc.wait();
                return Err("délai dépassé".into());
            }
        }
    }
    let o = proc.wait_with_output().map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

/// Mise à jour par git (avance rapide seulement). Rend vrai si le code a changé.
fn mise_a_jour() -> Result<bool, String> {
    // pas de git ou pas un dépôt : on démarre sans rien dire
    if git(&["rev-parse", "--is-inside-work-tree"], 5).is_err() {
        return Ok(false);
    }
    println!("[*] Recherche de mises à jour sur GitHub...");
    git(&["fetch", "--quiet", "origin"], 10).map_err(|e| format!("GitHub injoignable ({e})"))?;
    let amont = if git(&["rev-parse", "--verify", "@{u}"], 5).is_ok() {
        "@{u}"
    } else if git(&["rev-parse", "--verify", "origin/main"], 5).is_ok() {
        "origin/main"
    } else {
        return Ok(false);
    };
    let n: u32 = git(&["rev-list", &format!("HEAD..{amont}"), "--count"], 5)?
        .parse()
        .map_err(|_| "réponse de git illisible".to_string())?;
    if n == 0 {
        println!("[*] Le logiciel est à jour.");
        return Ok(false);
    }
    println!("MISE À JOUR DISPONIBLE ({n} nouveau(x) commit(s) sur GitHub)");
    let mut reserve = false;
    if !git(&["status", "--porcelain", "-uno"], 5)?.is_empty() {
        println!("  Modifications locales détectées : mise en réserve temporaire...");
        reserve = git(&["stash", "push", "-m", "Auto-stash WEB_RUGBY avant mise a jour"], 10).is_ok();
    }
    // --ff-only : un historique distant réécrit ou divergent est refusé, jamais fusionné en silence
    let fusion = git(&["merge", "--ff-only", amont], 120);
    if reserve {
        if let Err(e) = git(&["stash", "pop"], 10) {
            // le taire laisserait croire qu'elles sont perdues
            println!("[!] Vos modifications locales n'ont pas pu être réappliquées automatiquement :\n    {e}");
            println!("    Elles sont conservées dans la réserve Git : voir « git stash list », puis « git stash pop » une fois le conflit résolu.");
        }
    }
    fusion.map_err(|e| format!("échec de l'application de la mise à jour ({e})"))?;
    Ok(true)
}

pub fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match lire_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{USAGE}\nrugby: {e}");
            std::process::exit(2);
        }
    };
    if args.aide {
        println!("{USAGE}");
        return;
    }
    // --sans-pause : acceptée sans effet, la pause de fin vient de lancer.bat sous Windows
    let _ = std::env::set_current_dir(racine());
    if !args.sans_maj {
        match mise_a_jour() {
            Ok(true) if std::env::var("WEB_RUGBY_RELANCEUR").as_deref() == Ok("1") => {
                println!("Mise à jour appliquée : relance de l'outil...");
                std::process::exit(75);
            }
            Ok(true) => println!("Mise à jour appliquée : relancez l'outil pour en profiter."),
            Ok(false) => {}
            Err(e) => println!("[!] {e}\n[*] Démarrage avec la version actuelle..."),
        }
    }
    let reseau = !args.local;
    let srv = match Serveur::ouvrir(args.port, reseau, 10) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Aucun port libre entre {} et {} : {e}", args.port, args.port.saturating_add(9));
            std::process::exit(1);
        }
    };
    let _ = GARDE.set(Garde::new("WEB_RUGBY", reseau, None, None));
    garde().brancher(&srv); // en mode réseau : HTTPS sur le même port
    let port = srv.port();
    let url = format!("http://127.0.0.1:{port}/");
    println!("Rugby Top 14");
    println!("  adresse : {url}");
    if port != args.port {
        println!("  (le port {} était occupé)", args.port);
    }
    if reseau {
        for ip in appairage::adresses_poste().iter().filter(|ip| !ip.starts_with("127.") && *ip != "::1") {
            println!("  réseau  : {}   (tablette, autre poste)", garde().adresse(ip, port));
        }
        println!("            l'autre appareil demande une fois le code d'appairage ci-dessous");
        garde().annoncer();
    }
    println!("  Ctrl+C pour arrêter.\n");
    let _ = std::io::stdout().flush();
    if !args.sans_navigateur {
        ouvrir_navigateur(&url);
    }
    srv.servir(Arc::new(|r: &mut Requete| repondre(r)));
}

#[cfg(test)]
mod essais {
    use super::*;
    use std::io::Read;
    use std::net::TcpStream;

    fn a(l: &[&str]) -> Vec<String> {
        l.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn ligne_de_commande() {
        // la commande du catalogue de web_launcher
        let x = lire_args(&a(&["--port", "9000", "--sans-navigateur", "--sans-pause", "--sans-maj"])).unwrap();
        assert_eq!((x.port, x.sans_navigateur, x.sans_pause, x.sans_maj, x.local), (9000, true, true, true, false));
        assert!(lire_args(&a(&["--local"])).unwrap().local);
        assert_eq!(lire_args(&[]).unwrap().port, PORT);
        assert_eq!(lire_args(&a(&["--port=8800"])).unwrap().port, 8800);
        assert!(lire_args(&a(&["--port"])).is_err());
        assert!(lire_args(&a(&["--port", "x"])).is_err());
        assert!(lire_args(&a(&["--inconnu"])).is_err());
        assert!(lire_args(&a(&["--local=1"])).is_err());
    }

    /// (code, réponse brute) d'une requête au serveur de l'essai.
    fn demander(port: u16, methode: &str, chemin: &str, hote: Option<&str>) -> (u16, String) {
        let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let hote = hote.map(str::to_string).unwrap_or(format!("127.0.0.1:{port}"));
        write!(c, "{methode} {chemin} HTTP/1.0\r\nHost: {hote}\r\nContent-Length: 0\r\n\r\n").unwrap();
        let mut texte = String::new();
        c.read_to_string(&mut texte).unwrap();
        let code = texte.split(' ').nth(1).and_then(|x| x.parse().ok()).unwrap_or(0);
        (code, texte)
    }

    /// Repris de tests/banc-serveur.py : ce qu'il sert, ce qu'il refuse.
    #[test]
    fn fichiers_servis_et_refuses() {
        let _ = GARDE.set(Garde::new("WEB_RUGBY", false, None, None));
        let srv = Serveur::ouvrir(0, false, 1).unwrap();
        srv.servir_en_fond(Arc::new(|r: &mut Requete| repondre(r)));
        let port = srv.port();
        for chemin in ["/", "/index.html", "/index.html?saison=1", "/%69ndex.html"] {
            let (code, texte) = demander(port, "GET", chemin, None);
            assert_eq!(code, 200, "{chemin}");
            assert!(texte.contains("Cache-Control: no-store, max-age=0"), "{chemin}");
            assert!(texte.contains("Content-Security-Policy: frame-ancestors 'self'"), "{chemin}");
            assert!(texte.contains("X-Content-Type-Options: nosniff"), "{chemin}");
            assert!(texte.contains("<html") || texte.contains("<!DOCTYPE") || texte.contains("<!doctype"), "{chemin}");
        }
        assert_eq!(demander(port, "HEAD", "/", None).0, 200);
        for chemin in [
            "/.git/config", "/.git/HEAD", "/serveur.py", "/appairage.py", "/README.md", "/Cargo.toml",
            "/.gitignore", "/../README.md", "/%2e%2e/README.md", "/index.html/", "/tests/",
            "/squads/AB.json", "/css/style.css", "/js/app.js", "/js/../serveur.py", "/tests/banc-serveur.py",
            "/tests/audit.py", "/rust/rugby/src/serveur.rs", "/bin/rugby", "/.env",
        ] {
            assert_eq!(demander(port, "GET", chemin, None).0, 404, "{chemin}");
        }
        let (code, texte) = demander(port, "GET", "/", Some("piege.example"));
        assert_eq!(code, 403, "Host inconnu");
        assert!(texte.contains("X-Content-Type-Options: nosniff"));
        assert_eq!(demander(port, "POST", "/", None).0, 501, "POST");
        assert_eq!(demander(port, "PUT", "/index.html", None).0, 501, "PUT");
        assert_eq!(demander(port, "DELETE", "/index.html", None).0, 501, "DELETE");
        srv.arreter();
    }
}

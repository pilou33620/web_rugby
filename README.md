# web_rugby

Simulateur de rugby autonome du Top 14 (saison 2026-2027), avec un rendu en hexagones et en vue isométrique. Ce n'est pas un jeu : les matchs se jouent seuls et les équipes gagnent en expérience au fil des victoires et des défaites.

Tout tient dans un seul fichier : `index.html` (HTML, CSS et JavaScript, sans dépendance). Il suffit de l'ouvrir dans un navigateur.

## Installation

### Le plus simple : l'archive de production (sans git, sans rien installer)

Dans la [publication « binaires »](https://github.com/pilou33620/web_rugby/releases/tag/binaires), prendre l'archive du système :

| Système | Archive |
|---|---|
| Windows | `rugby-prod-windows-x86_64.zip` |
| Linux x86_64 | `rugby-prod-linux-x86_64.tar.gz` |
| Raspberry Pi (64 bits) | `rugby-prod-linux-aarch64.tar.gz` |
| macOS (Apple) | `rugby-prod-macos-aarch64.tar.gz` |

La décompresser où l'on veut (elle donne un dossier `rugby/` : le serveur dans `bin/`, `index.html`, les scripts de démarrage), puis double-cliquer sur `demarrer_WEB_RUGBY.cmd` (Windows) ou lancer `./demarrer_WEB_RUGBY.sh`. À chaque démarrage, le serveur compare son archive à la dernière publiée ; si elle a changé, il la télécharge (empreinte SHA-256 vérifiée), recopie ses fichiers par-dessus et redémarre. La saison vit dans le navigateur : une mise à jour ne la touche pas.

### Pour développer : le dépôt git

```bash
git clone https://github.com/pilou33620/web_rugby.git
cd web_rugby
./demarrer_WEB_RUGBY.sh             # Windows : demarrer_WEB_RUGBY.cmd
```

`demarrer_WEB_RUGBY` obtient d'abord le serveur (`scripts/binaires.sh`, `scripts/binaires.cmd`) : sans `cargo`, il télécharge l'exécutable du système (publication GitHub « binaires », compilée par la CI à chaque envoi sur `main`, vérifiée par son empreinte SHA-256) dans `bin/` ; avec `cargo`, il est compilé sur place. À la racine, seul le lanceur du système en cours reste visible : le premier lancement masque l'autre. Sur une erreur, la fenêtre reste ouverte avec la raison. `lancer.sh` et `lancer.bat` restent, pour web_launcher : ils relaient vers `demarrer_WEB_RUGBY` avec les mêmes options.

## Lancer

- **Par web_launcher** : carte Rugby Top 14 (port 8796). Depuis un autre appareil, ouvrez l'adresse par l'IP du Pi ; le code d'appairage demandé une fois s'affiche sur la carte, et HTTPS demande d'installer la CA (`/appairage/ca.pem`).
- **Avec le serveur** : `./demarrer_WEB_RUGBY.sh` (Linux, macOS, Raspberry Pi) ou `demarrer_WEB_RUGBY.cmd` (Windows), voir [Installation](#installation). Le serveur est en Rust ; Rust n'est pas nécessaire. Il ne sert que `index.html` et ne stocke rien.
- **Localement** : ouvrez `index.html` dans un navigateur.

Options du serveur :

| Option | Rôle |
|---|---|
| `--port N` (ou `--port=N`) | port d'écoute, 8796 par défaut (le suivant libre si occupé, 10 essais) |
| `--local` | n'écouter que sur `127.0.0.1`, sans code d'appairage ; sinon réseau local et Tailscale, derrière l'appairage commun aux web tools |
| `--sans-navigateur` | ne pas ouvrir le navigateur |
| `--sans-pause` | acceptée pour le launcher (sous Windows, `demarrer_WEB_RUGBY.cmd` garde la fenêtre ouverte sur une erreur) |
| `--sans-maj` | ne pas chercher de mise à jour sur GitHub au démarrage |

Sans `--sans-maj`, le serveur met à jour un clone git par `git fetch` puis une avance rapide (`git merge --ff-only`), une archive de production par la publication « binaires » ; s'il s'est mis à jour, il sort avec le code 75 et `demarrer_WEB_RUGBY` reprend l'exécutable de la nouvelle version puis le relance.

Tests : `cargo test` (options du catalogue, page servie, fichiers refusés, Host inconnu, méthodes d'écriture refusées, en-têtes). La CI GitHub les lance à chaque envoi.

## Fonctionnalités

- **Les 14 clubs 2026-2027** avec leurs vrais effectifs (30 joueurs par club) et leurs statistiques : passe, mêlée, attaque, défense, rapidité, touche, jeu au pied, discipline.
- **Match autonome d'environ 9 min 30 à ×1** (vitesses ×½, ×1, ×2, ×4, ⏭).
- **Règles** :
  - placements de départ, coup d'envoi à 10 m ;
  - rucks avec hors-jeu, mêlées, touches, mauls ;
  - en-avant, passes vers l'arrière uniquement ;
  - pénalités (au but, pénaltouche, jeu rapide), transformations, drops ;
  - 50/22, renvoi aux 22 m ;
  - sirène, avec le jeu qui continue jusqu'au ballon mort.
- **Joueurs** : fatigue, remplacements à partir de la 50e minute, cartons jaunes et rouges (jeu à 14), blessures et infirmerie qui persiste d'un match à l'autre.
- **Contexte** :
  - avantage du terrain (forteresses) ;
  - météo (pluie, vent, chaleur, brouillard, froid) avec effet sur le jeu et à l'écran ;
  - saison et terrain lourd ou sec, nocturnes, Tournoi des Six Nations ;
  - profil d'arbitre (arbitres fictifs).
- **Stratégies** : 4 plans d'attaque (avants, large, au pied, mouvement) et 3 plans de défense (glissée, rush, grattage).
- **Combinaisons de jeu** : croisée, redoublement, intrusion de l'arrière, ailier qui rentre, leurre, départ du n°8, ballon porté simulé. Les courses prévues sont tracées en pointillés, les équipes apprennent quelles combinaisons marchent pour elles, et la défense peut les lire.
- **Apprentissage par renforcement** : chaque possession met à jour la valeur des plans selon la zone du terrain. S'y ajoutent de l'expérience (XP) et une progression des statistiques après chaque match.
- **Championnat** : 26 journées, barème du Top 14 avec bonus, barrages, demi-finales et finale, palmarès.
- **Lecture du jeu** :
  - flèches de passe, trajectoires des coups de pied, ligne d'avantage ;
  - courses de tous les joueurs affichées en permanence (bouton 🧭 Courses) ;
  - nom du porteur, mini-carte, réalisation télé ;
  - ralenti automatique sur les essais.
- **Ambiance** : son du stade généré par le navigateur et voix du commentateur (synthèse vocale).

La mémoire (expérience, classement, infirmerie) est gardée dans le navigateur (`localStorage`).

## Structure

| Chemin | Contenu |
|---|---|
| `index.html` | Le simulateur complet. Les effectifs y sont intégrés. |
| `squads/*.json` | Effectifs sources par club : nom, poste, titulaire, international, sources. |
| `rust/rugby/` | Serveur pour web_launcher et le réseau local (Rust) ; sécurité réseau prise dans la caisse commune `web_tools_commun` (dépôt web_suite). |
| `demarrer_WEB_RUGBY.sh`, `demarrer_WEB_RUGBY.cmd`, `scripts/` | Obtiennent l'exécutable (compilé ou téléchargé) puis le lancent. |
| `lancer.sh`, `lancer.bat` | Relais vers `demarrer_WEB_RUGBY`, pour web_launcher. |
| `rust/audit/` | Audit du simulateur dans Chromium (outil de développement, voir plus bas). |

## Audit du simulateur

Audit du simulateur (outil de développement) : la caisse `rust/audit` joue N matchs dans Chromium, piloté par le protocole DevTools (`headless_chrome`), sans serveur (elle ouvre `index.html` directement), et contrôle l'intégrité de la grille, les règles et le réalisme des statistiques. Elle affiche le bilan et sort en erreur s'il y a une violation ou une erreur dans la page. Depuis la racine du dépôt :

```sh
cargo run -p audit -- 40                       # 40 matchs (par défaut)
CHROME=/chemin/vers/chrome cargo run -p audit -- 40
```

Sans `CHROME`, elle prend le Chromium de Playwright (`/opt/pw-browsers`) s'il existe, sinon `chrome` ou `chromium` dans le PATH. L'outil n'entre ni dans `bin/` ni dans les archives publiées, et `cargo build` à la racine ne compile que le serveur. La CI le compile et le fait tourner sur 3 matchs.

## Limites connues

- Les noms des joueurs viennent des présentations de saison 2026-2027 (RugbyPass). Le choix titulaire / remplaçant est une estimation.
- La mention « international » est souvent manquante, ce qui rend l'effet du Tournoi inégal selon les clubs.
- Le match est comprimé : environ 3 essais et 40 points par match en moyenne, contre 4 à 5 essais en vrai.

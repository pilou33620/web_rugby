# web_rugby

Simulateur de rugby autonome du Top 14 (saison 2026-2027), avec un rendu en hexagones et en vue isométrique. Ce n'est pas un jeu : les matchs se jouent seuls et les équipes gagnent en expérience au fil des victoires et des défaites.

Tout tient dans un seul fichier : `index.html` (HTML, CSS et JavaScript, sans dépendance). Il suffit de l'ouvrir dans un navigateur.

## Lancer

- **Par web_launcher** : carte Rugby Top 14 (port 8796). Depuis un autre appareil, ouvrez l'adresse par l'IP du Pi ; le code d'appairage demandé une fois s'affiche sur la carte, et HTTPS demande d'installer la CA (`/appairage/ca.pem`).
- **Avec le serveur** : `python3 serveur.py [--port 8796] [--local] [--sans-navigateur] [--sans-pause] [--sans-maj]`. Python 3.8+, bibliothèque standard seulement. Sans `--sans-maj`, il se met à jour depuis GitHub au démarrage. Il ne sert que `index.html` et ne stocke rien.
- **Localement** : ouvrez `index.html` dans un navigateur.

Bancs d'essai : `python3 tests/banc-serveur.py` et `python3 appairage.py`, qui affichent `OK`.

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
| `serveur.py`, `appairage.py` | Serveur pour web_launcher et le réseau local ; `appairage.py` est une copie de celui de web_suite. |
| `tests/banc-serveur.py` | Banc du serveur : ce qu'il sert, ce qu'il refuse. |
| `tests/audit.py` | Audit automatique : joue N matchs sans affichage et contrôle l'intégrité et le réalisme. |

## Lancer l'audit

```bash
pip install playwright && playwright install chromium
python3 tests/audit.py 40
```

## Limites connues

- Les noms des joueurs viennent des présentations de saison 2026-2027 (RugbyPass). Le choix titulaire / remplaçant est une estimation.
- La mention « international » est souvent manquante, ce qui rend l'effet du Tournoi inégal selon les clubs.
- Le match est comprimé : environ 3 essais et 40 points par match en moyenne, contre 4 à 5 essais en vrai.

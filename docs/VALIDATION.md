# Validation de FilePilot 0.1.0 — 13 octobre 2026

Cette version fournit le moteur et l'installateur initial. Elle n'est pas déclarée conforme à toute la Definition of Done d'une V1 stable.

Installateur livré : `FilePilot_0.1.0_x64-setup.exe`, **4 860 789 octets**. SHA-256 : `267AFDF9E2415835DE80377A92CF1FA757ABBCC1D2DEB91C193EE7DDC5C4A14C`. Copié dans le dossier `outputs` de la session. Les cycles silencieux d'installation/désinstallation ont été testés sur les builds 0.1.0 précédant les derniers ajustements de fermeture et d'accessibilité ; le dernier binaire a aussi été lancé et son interface native lue avec Computer Use.

## Résultats vérifiés localement

| Contrôle | Résultat |
| --- | --- |
| `npm run lint` | Réussi, aucune erreur ni avertissement ESLint |
| `npm test` | 5 tests réussis |
| `npm run build` | TypeScript strict et Vite réussis ; éditeur chargé séparément |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Réussi |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | Réussi |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 35 tests réussis ; benchmark ignoré dans cette commande |
| `npx playwright test` | 6 tests réussis à 390, 720 et 1180 px, contraste du bouton principal et retour de focus compris |
| `npm run build:windows` | Chaîne complète réussie, installateur NSIS généré |
| Refus de publication sans clés | Vérifié : `release.mjs configure` échoue explicitement |
| Versions des manifests | Vérifiées, y compris environnement CI de branche |
| Installation silencieuse par utilisateur | Réussie dans un dossier de test sous `work/` |
| Binaire installé | Processus actif, fenêtre `FilePilot` répondante, aucune sortie d'erreur de démarrage |
| Instance unique | Deuxième lancement termine avec code 0 ; un seul processus demeure |
| Intégration d'installation | Registre version 0.1.0 et raccourci du menu Démarrer présents |
| Désinstallation silencieuse | Réussie ; programme, registre et raccourci retirés |
| Conservation des données | SQLite et fichier personnel témoin conservés après désinstallation |

Les tests navigateur utilisent des fixtures IPC isolées pour vérifier les composants et les interactions. Ils ne simulent aucune opération de production dans le logiciel livré et ne constituent pas une validation des interactions natives Tauri.

Les tests Rust réalisent de véritables déplacements dans des dossiers temporaires. Ils couvrent simulation, conditions/ET/OU/glob/exclusions/priorité, règles inactives, chemins protégés et alias Windows, Unicode, collisions et concurrence sur la destination, fichier disparu/verrouillé, création de destination, renommage, historique SQLite, reprise après interruption, annulation après modification externe, annulation groupée, purge, suppression des boucles, surveillance réelle et conservation du flux NTFS `Zone.Identifier`.

## Mesures

Benchmark isolé : 10 000 fichiers PDF synthétiques de 17 octets, build Rust de développement. Génération : **1 467 ms**. Analyse avec calcul BLAKE3 : **733 ms**. Résultat sérialisé des propositions : **4 517 781 octets**. Le premier moteur prenait 10 662 ms avant suppression des validations redondantes par fichier.

Le processus natif installé au repos utilisait environ **40,2 Mio** de working set. Cette mesure exclut les processus enfants WebView2 et ne représente pas la consommation pendant le benchmark. Débit sur de gros fichiers, mémoire totale sous charge et réactivité native pendant 10 000 opérations n'ont pas été mesurés. Aucun chiffre n'est extrapolé à ces cas.

## Limites de validation et de livraison

- Le contrôle automatique a refusé le lancement avec débogage WebView2 (`blocked by policy`). Le démarrage normal et les cycles silencieux d'installation ont pu être testés. Computer Use a lu l'interface native initiale (onboarding, navigation, compteurs réels et version), confirmant la connexion IPC. L'activation échoue toutefois deux fois avec `failed to activate captured window` ; une fenêtre Armoury Crate Update recouvre FilePilot. Le parcours natif complet, les menus de zone de notification, l'autostart et les notifications Windows restent à vérifier.
- Les clés de signature de production ne sont pas fournies. L'updater est disponible dans les builds signés préparés par le pipeline, mais le build local n'intègre aucun updater de production. Aucun test de mise à jour signée entre deux releases publiques, de signature corrompue sur le runtime installé ou de signature Authenticode n'est déclaré réussi.
- L'installateur interactif par double-clic et les choix de ses pages n'ont pas été parcourus ; le test utilise `/S` et un chemin isolé.
- Les opérations inter-volumes utilisent la même copie Windows vérifiée, mais n'ont pas été testées sur un second volume physique dans cette session.
- Les fichiers en lecture seule, points de jonction, liens, chemins réseau et fichiers cloud/reparse sont exclus. Les opérations ambiguës nécessitent inspection humaine.
- L'accueil accompagne les vues principales plutôt qu'un assistant modal en cinq étapes ; les raccourcis avancés et l'action Copier ne sont pas livrés. Certains détails d'erreurs techniques restent en anglais. L'interface charge au maximum 10 000 opérations.
- Les sources de règles importées doivent correspondre à des dossiers déjà enregistrés ; aucun remappage interactif inter-machine n'est implémenté.
- Les workflows sont présents. Leur résultat distant est à consulter dans GitHub Actions ; la compilation locale ne prouve pas un succès distant.

## Reprise recommandée

Valider le parcours natif sur un compte/VM Windows isolé, puis configurer les clés et l'environnement GitHub `release` selon `RELEASE.md`. Tester une mise à jour entre deux versions avec la même clé et le refus d'une signature modifiée avant de publier une version stable. Ajouter ensuite les mesures sous charge et le remappage d'import selon les besoins réels.

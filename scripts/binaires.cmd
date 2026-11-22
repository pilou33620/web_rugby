@echo off
rem Met l'executable de l outil dans bin\ (appele par demarrer_WEB_RUGBY.cmd, depuis la racine).
rem Avec cargo sur le poste : compile ici, toujours a jour avec les sources.
rem Sinon : telecharge depuis GitHub (publication "binaires", compilee par la CI a chaque
rem envoi sur main), une fois par version : bin\commit dit de quelle version il vient.
setlocal
rem archive de production (publication "binaires", sans les sources) : son executable est deja la
if not exist Cargo.toml if exist bin\rugby.exe exit /b 0
if not exist bin mkdir bin
where cargo >nul 2>nul
if not errorlevel 1 (
  cargo build --release -p rugby --bins || exit /b 1
  copy /y target\release\*.exe bin\ >nul 2>nul
  exit /b 0
)
set ICI=
for /f %%c in ('git rev-parse HEAD 2^>nul') do set ICI=%%c
set LA=
if exist bin\commit set /p LA=<bin\commit
if exist bin\rugby.exe if defined ICI if "%ICI%"=="%LA%" exit /b 0
echo   Telechargement de l outil (windows-x86_64)...
curl -fsSL -o bin\.archive.zip https://github.com/pilou33620/web_rugby/releases/download/binaires/rugby-windows-x86_64.zip || goto echec
curl -fsSL -o bin\.empreinte https://github.com/pilou33620/web_rugby/releases/download/binaires/rugby-windows-x86_64.zip.sha256 || goto echec
rem l'archive doit avoir l'empreinte SHA-256 publiee avec elle : tronquee ou alteree en route, elle est refusee
set ATTENDU=
set /p ATTENDU=<bin\.empreinte
set RECU=
for /f "skip=1 delims=" %%h in ('certutil -hashfile bin\.archive.zip SHA256') do if not defined RECU set RECU=%%h
set RECU=%RECU: =%
if /i not "%RECU%"=="%ATTENDU%" (
  echo   [!] Empreinte de l'archive incorrecte : archive refusee.
  goto echec
)
tar -xf bin\.archive.zip -C bin || goto echec
del bin\.archive.zip bin\.empreinte >nul 2>nul
exit /b 0
:echec
del bin\.archive.zip bin\.empreinte >nul 2>nul
if exist bin\rugby.exe (
  echo   [!] Telechargement impossible : on garde l'executable deja present.
  exit /b 0
)
echo [X] Telechargement de l outil impossible.
echo     Installer Rust (https://rustup.rs) puis relancer : il sera compile ici.
exit /b 1

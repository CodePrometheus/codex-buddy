<p align="center">
  <img src="docs/logo.png" width="120" alt="codex-buddy" />
</p>

<h1 align="center">codex-buddy</h1>

<p align="center">
  Una forma <b>pequeña y rápida</b> de ejecutar varias cuentas de <a href="https://developers.openai.com/codex">Codex CLI</a> en paralelo —<br/>
  un binario de <b>544 KB</b>, cambia o corre en simultáneo, sin re-logins, local por defecto.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" />
  <img src="https://img.shields.io/badge/rust-1.89%2B-orange.svg" alt="Rust" />
  <img src="https://img.shields.io/badge/platform-macOS-lightgrey.svg" alt="Platform" />
  <img src="https://img.shields.io/badge/binary-544K-brightgreen.svg" alt="Binary" />
</p>

<p align="center">
  <a href="README.md">English</a> | <a href="README.zh-CN.md">简体中文</a> | <b>Español</b>
</p>

## Características

- **Pequeña y rápida** — un único binario de 544 KB, solo 4 dependencias directas, cero async /
  cero HTTP / cero crypto. Cambiar de cuenta es un `rename` atómico (**instantáneo**); detectar qué
  cuentas están corriendo en paralelo usa una syscall nativa (~**2 ms**). El binario de release se
  comprime con `opt-level=z` + `lto` + `strip`.
- **Cuentas realmente en paralelo** — ejecuta dos o más sesiones de Codex **al mismo tiempo**, cada
  una con su propia cuenta, totalmente aisladas.
- **Nunca fuerza un nuevo login** — cambia de cuenta ida y vuelta las veces que quieras, sin
  cierre de sesión forzado ni riesgo de activar la detección de abuso.
- **Local por defecto** — sin telemetría y con una CLI sin código de red. El uso en vivo es
  estrictamente opt-in, e incluso entonces quien hace la petición es codex — ver
  [Uso en vivo](#uso-en-vivo-opcional). La única llamada de red propia de la app de la barra de
  menú es la comprobación de actualizaciones que tú mismo activas.
- **Segura por diseño** — la configuración inicial respalda tu sesión existente antes de tocarla y
  revierte ante cualquier fallo; un solo comando `doctor` te dice si algo no está bien.
- **Config compartida, logins aislados** — `config.toml` y las reglas aplican a todas las
  cuentas; las credenciales nunca se filtran entre cuentas.

## App en la barra de menú

Además de la CLI, codex-buddy incluye una app nativa en la barra de menú de macOS: haz clic en el
icono y un panel muestra el uso de cada cuenta, cuál está activa y cuáles corren en paralelo — un
clic para cambiar. **Igual de pequeña** — un bundle de una sola arquitectura pesa menos de 1 MB.

<p align="center">
  <img src="docs/panel-light.png" width="380" alt="Panel (claro)" />
  <img src="docs/panel-dark.png" width="380" alt="Panel (oscuro)" />
</p>

- **Anillos de uso** — cuánto queda en cada ventana de límite que codex reporta, con color según
  el umbral.
- **Uso en vivo (opt-in)** — el rayo de la cabecera activa la consulta de números actuales a
  través de codex al abrir el panel (con limitación de frecuencia, en paralelo por cuenta); los
  datos locales siguen siendo el respaldo. Desactivado por defecto.
- **Lista de cuentas** — avatar de color propio por cuenta, insignia de plan, punto verde de
  ejecución en paralelo y una marca en la cuenta activa.
- **Doctor integrado** — autochequeo en el propio panel; despliega una lista solo cuando algo está
  mal, con copia del informe en un clic.
- **Claro / oscuro** — sigue al sistema, o alterna claro / oscuro tú mismo.
- **Acciones en línea + Añadir cuenta** — una fila de iconos por cuenta para renombrar, copiar
  `CODEX_HOME`, ejecutar en Terminal o eliminar; "Add Account" se despliega en el sitio, lanzando un
  `codex login` real o importando un `auth.json` existente.

<p align="center">
  <img src="docs/actions.png" width="380" alt="Acciones en línea y añadir cuenta" />
</p>

- **Elemento de estado en la barra de menú** — mira la cuenta activa y su porcentaje de uso más
  ajustado sin siquiera abrir el panel, con color según el umbral.

<p align="center">
  <img src="docs/menubar.png" width="220" alt="Elemento de estado en la barra de menú" />
</p>

Descarga la app desde [Releases](https://github.com/CodePrometheus/codex-buddy/releases):
`Codex-Buddy-arm64-macOS.zip` para Apple Silicon, `Codex-Buddy-x86_64-macOS.zip` para Intel. No
está firmada, así que la primera apertura requiere clic derecho → Abrir.

## Instalación

**Homebrew.**

```sh
brew install CodePrometheus/tap/codex-buddy
```

**Script de shell.** Descarga un binario precompilado, sin necesidad de Homebrew:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/CodePrometheus/codex-buddy/releases/latest/download/codex-buddy-installer.sh | sh
```

Ambos requieren macOS con Apple Silicon o Intel; en [Releases](https://github.com/CodePrometheus/codex-buddy/releases) hay binarios precompilados y checksums.

## Inicio rápido

```
$ codex-buddy init
Detected current account:
  email : alice@work.example
  plan  : plus

Alias for this account [work]:
...
Done: account 'work' is managed and set as current.

$ codex-buddy add personal
Opening codex login for 'personal'; complete the login in your browser...
...
Account 'personal' added. Use `codex-buddy switch personal`, or `codex-buddy run personal -- ...`
to run it in parallel.

$ codex-buddy list
  ALIAS     EMAIL                   PLAN  1W        LAST USED
* work      alice@work.example      plus  12% (4d)  just now
  personal  alice@personal.example  pro   0% (6d)   2d ago

$ codex-buddy switch personal
Switched to: personal  alice@personal.example  [pro]

$ codex
# arranca de inmediato, sin pedir login

$ codex-buddy switch -
Switched to: work  alice@work.example  [plus]
```

Ejecuta dos cuentas lado a lado sin cambiar ninguna:

```
# terminal 1
$ codex-buddy run work -- codex

# terminal 2
$ codex-buddy run personal -- codex
```

## Comandos

**Configuración**

| Comando | Descripción |
|---|---|
| `init [alias] [--yes]` | Adopta la cuenta actual de `~/.codex` |
| `add <alias>` | Inicia sesión y adopta una cuenta nueva |
| `import <auth.json> [--alias a] [--json]` | Importa una cuenta |
| `import <directorio> [--skip-existing] [--json]` | Importa hijos directos `<alias>/auth.json`; los éxitos se conservan |
| `export <alias> <ruta> [--force]` | Exporta un archivo de credenciales con permisos `0600` |
| `relogin <alias>` | Reinicia sesión en una cuenta existente (p. ej. tras expirar el token) |
| `rename <viejo> <nuevo>` | Renombra una cuenta |
| `remove <alias> [--yes]` | Elimina una cuenta (rechaza eliminar la activa) |

**Uso**

| Comando | Descripción |
|---|---|
| `list [--json]` | Lista las cuentas con su uso |
| `current [--json]` | Muestra la cuenta activa |
| `usage [alias] [--remote] [--json]` | Muestra el uso y si está fresh, expired o missing |
| `recommend [--remote] [--json]` | Recomienda la cuenta con más margen |
| `switch <alias> \| - \| --next` | Cambia de cuenta (`-` = anterior, `--next` = rota en orden de registro) |
| `run <alias> -- <args>` | Ejecuta codex bajo una cuenta, en paralelo |
| `path <alias>` | Imprime el `CODEX_HOME` de una cuenta |
| `doctor [--json]` | Comprueba la salud de la instalación |
| `report [--json]` | Resume cuentas y comprobaciones de salud |

Las tablas de uso muestran una columna por cada ventana de límite presente en los datos — el
conjunto de ventanas ha cambiado upstream antes, así que no hay ninguna columna fija.

La importación de directorios escanea deliberadamente un solo nivel:

```
accounts/
├── work/auth.json
└── personal/auth.json
```

Cada cuenta se confirma de forma independiente. El comando imprime `imported`, `skipped` o
`failed` por cada elemento, conserva las importaciones exitosas y sale con código distinto de
cero si algo falló. `--skip-existing` solo omite un alias existente con la misma identidad de
cuenta; nunca reemplaza credenciales.

Los `auth.json` exportados contienen tokens de acceso y de refresco. codex-buddy los crea con
`0600`, rechaza destinos symlink o dentro del gestionado `~/.codex-buddy`, y exige `--force` para
reemplazar un archivo regular existente.

Codex debe guardar tu login como archivo plano, no en el llavero del sistema — codex-buddy
gestiona ese archivo directamente, así que necesita tenerlo en disco. `init` y `add` lo comprueban
automáticamente y te dicen cómo arreglarlo (`cli_auth_credentials_store = "file"` en
`~/.codex/config.toml`) si no es así.

## Uso en vivo (opcional)

Por defecto, cada número que codex-buddy muestra sale de los datos de sesión locales: correctos a
fecha de la última vez que esa cuenta ejecutó codex, y etiquetados honestamente como `fresh` /
`expired` / `missing`. Cuando quieras números actuales:

```
$ codex-buddy usage --remote
  ALIAS     STATUS  1W
  work      fresh   15% (5d)
  personal  fresh   6% (4d)

$ codex-buddy recommend --remote
Recommended: personal
  bottleneck: 1w with 94% remaining
  1w: 6% used, 94% remaining, resets in 4d
```

`--remote` se lo pregunta a codex: codex-buddy arranca `codex app-server` con el `CODEX_HOME` de
cada cuenta, lee los límites de esa cuenta por su protocolo stdio y lo detiene. La petición de
red la hace el binario oficial de codex — su propio cliente, su propia autenticación, su propio
refresco de tokens. codex-buddy nunca habla directamente con ningún backend, no contiene código
HTTP y no envía nada a ninguna parte; eso sigue siendo cierto con `--remote`.

La app de la barra de menú tiene el mismo interruptor tras el icono del rayo ("Live usage via
codex"), desactivado por defecto y persistente una vez lo actives.

## Licencia

[MIT License](LICENSE)

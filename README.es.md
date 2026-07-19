# codex-buddy

[English](README.md) | [简体中文](README.zh-CN.md) | **Español**

![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.89%2B-orange.svg)
![Platform](https://img.shields.io/badge/platform-macOS-lightgrey.svg)

Ejecuta varias cuentas de [Codex CLI](https://developers.openai.com/codex) en paralelo — cambia
o corre en simultáneo, sin re-logins.

## Características

- **Cuentas realmente en paralelo** — ejecuta dos o más sesiones de Codex al mismo tiempo, cada
  una con su propia cuenta
- **Nunca fuerza un nuevo login** — cambia de cuenta ida y vuelta las veces que quieras, sin
  cierre de sesión forzado ni riesgo de activar la detección de abuso
- **100% local** — sin telemetría, sin dependencia de la nube, nada sale de tu máquina; un único
  binario de menos de 1&nbsp;MB
- **Segura por diseño** — la configuración inicial respalda tu sesión existente antes de tocarla y
  revierte ante cualquier fallo; un solo comando `doctor` te dice si algo no está bien
- **Config compartida, logins aislados** — `config.toml` y las reglas aplican a todas las
  cuentas; las credenciales nunca se filtran entre cuentas

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
  ALIAS      EMAIL                  PLAN  5H  1W       ACTIVE
* work       alice@work.example     plus  -   12% (4d)  just now
  personal   alice@personal.example pro   -   0% (6d)   2d ago

$ codex-buddy switch personal
Switched to: personal  alice@personal.example  [pro]

$ codex
# arranca de inmediato, sin pantalla de login

$ codex-buddy switch -
Switched to: work  alice@work.example  [plus]
```

Ejecuta dos cuentas en paralelo, en dos terminales, sin cambiar ninguna de las dos:

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
| `import <path> [--alias a]` | Adopta una cuenta a partir de un `auth.json` existente |
| `relogin <alias>` | Vuelve a iniciar sesión en una cuenta existente |
| `rename <old> <new>` | Renombra una cuenta |
| `remove <alias> [--yes]` | Elimina una cuenta (rechaza eliminar la cuenta activa) |

**Uso**

| Comando | Descripción |
|---|---|
| `list` | Lista las cuentas con su uso |
| `current` | Muestra la cuenta activa |
| `switch <alias> \| -` | Cambia de cuenta (`-` = la anterior) |
| `run <alias> -- <args>` | Ejecuta codex bajo una cuenta, en paralelo |
| `path <alias>` | Imprime el `CODEX_HOME` de una cuenta |
| `doctor` | Verifica el estado de la instalación |

Codex debe guardar tu sesión como un archivo normal, no en el llavero del sistema — codex-buddy
gestiona ese archivo directamente, así que lo necesita en disco. `init` y `add` lo comprueban
automáticamente y te dicen cómo arreglarlo (`cli_auth_credentials_store = "file"` en
`~/.codex/config.toml`) si no es así.

## Licencia

[MIT License](LICENSE)

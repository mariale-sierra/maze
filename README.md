# Backrooms Raycaster

video del juego:

https://github.com/user-attachments/assets/e9bcb94c-144e-47d6-be67-835544414b55



## Estructura del proyecto

```
src/
├── main.rs         # loop principal, estados del juego, render de escena, HUD
├── maze.rs         # mapa del nivel y algoritmo de raycasting (DDA)
├── texture.rs       # generación procedural de texturas de pared
├── framebuffer.rs   # buffer de píxeles en RAM antes de subir a GPU
├── target.rs         # sprites (blancos), spawn, animación y disparo
└── imgutil.rs        # helper para crear imágenes en blanco (compat raylib 6.0)
```

---

## 1. El patrón de rombos en las paredes

Las texturas **no son imágenes cargadas desde archivo** — se generan por
código, píxel por píxel, en `texture.rs` (función `generate_wallpaper_image`).

La idea es que recorre una textura de `64x64` píxeles y se decide el
color de cada píxel según su posición dentro de una celda repetida de
`16x16`:

Cada tipo de pared (`1`, `2`, `3`, `4`, `g` en el mapa) usa la misma función
pero con una combinación distinta de `base`/`dark`, así se logran varias
paredes visualmente diferenciadas sin necesitar assets externos.

```rust
let wall_defs: Vec<(char, Color, Color)> = vec![
    ('1', Color::new(196, 178, 74, 255), Color::new(150, 132, 46, 255)),
    ('2', Color::new(180, 160, 70, 255), Color::new(130, 112, 40, 255)),
    // ...
];
```

Una vez generado el arreglo de colores en RAM, se sube como textura de GPU
(`load_texture_from_image`) — aunque en el render por columnas realmente se
consulta el arreglo de píxeles en CPU (`TextureManager::sample`), porque el
raycaster dibuja manualmente al framebuffer, no usa la textura de GPU para
esto.

---

## 2. La iluminación (parpadeo + niebla)

En `main.rs`, función `render_scene`, hay dos efectos de luz combinados que aparecen en la pantalla final o de exito 
y en las paredes del maze:

**Parpadeo (`flicker`)**: se calcula sumando dos ondas seno de distinta
frecuencia, aparece mas que todo en la pantalla final

**Niebla por distancia (`fog`)**: entre más lejos está una pared, más se
mezcla su color hacia un tono oscuro casi negro:

También se aplica una sombra fija según el lado de la pared golpeada
(`side_shade`), para que las paredes que dan hacia un eje se vean un poco
más oscuras que las del otro eje para que el ojo perciba mejor los bordes/esquinas del laberinto.

---

## 4. Sprites: los objetivos (`target.rs`)

Los "enemigos" son sprites planos que
siempre miran de frente a la cámara)

1. Se calcula el ángulo entre jugador y objetivo con `atan2`.
2. Se compara contra el ángulo de visión del jugador y su FOV, para saber
   si el sprite cae dentro de lo que se está renderizando.
3. Su tamaño en pantalla se escala según la distancia (`screen_height /
   distancia`) — igual que las paredes, más cerca = más grande.
4. **Oclusión**: antes de dibujar el sprite se reutiliza `cast_ray` desde
   el jugador hacia el objetivo. Si esa pared está más cerca que el
   objetivo, significa que hay algo bloqueando la vista, así que no se
   dibuja — esto evita que el objetivo se vea "a través" de las paredes.
5. La silueta en sí (`is_silhouette`) no es una imagen — es geometría
   simple: un círculo para la cabeza (`dx² + dy² < r²`) y un trapecio que
   se ensancha hacia abajo para el cuerpo.
6. **Animación**: se le suma un desplazamiento vertical basado en
   `sin(time * velocidad)`, haciendo que el sprite "flote" sutilmente en
   cada frame — esta es la animación de sprite requerida por la rúbrica.

El disparo (`try_shoot`) usa la misma lógica de ángulo + distancia +
oclusión, pero con una tolerancia de ángulo mucho más estrecha, simulando
una mira de precisión: si el jugador apunta casi exactamente al objetivo y
no hay pared en medio, el objetivo "muere".

---

## 5. Framebuffer manual

En vez de dibujar pixel por pixel directamente a la pantalla con llamadas
de raylib se pinta a una `Image` en RAM (`framebuffer.rs`) y **al final del frame completo** esa imagen se sube
de una sola vez a la GPU como textura, dibujándose como un único rectángulo
estirado a pantalla completa. Asi como se explico en la parte de texturas al inicio


---

## 6. Rotación con mouse

Se usa `get_mouse_delta().x` (cuánto se movió el mouse en X desde el frame
anterior).
El cursor se oculta y se fija al centro de la ventana (`disable_cursor`)
mientras se está jugando, y se libera de nuevo en las pantallas de
bienvenida y éxito.

---

## 7. Colisión y movimiento

`Maze::collides` revisa las 4 esquinas de un cuadrado alrededor del
jugador (su radio de colisión) y pregunta si alguna cae sobre una celda de
pared. El movimiento en X y en Y se prueban **por separado** en `main.rs`:

```rust
let try_x = player.x + move_x;
if !maze.collides(try_x, player.y, PLAYER_RADIUS) { player.x = try_x; }
let try_y = player.y + move_y;
if !maze.collides(player.x, try_y, PLAYER_RADIUS) { player.y = try_y; }
```

Esto permite que el jugador se **deslice** a lo largo de una pared en vez
de quedar completamente detenido cuando choca en diagonal — si el
movimiento falla en un eje pero no en el otro, igual avanza parcialmente.

---

## 8. Minimapa

Dibujado directamente con funciones de raylib
(`draw_rectangle`, `draw_circle`, `draw_line`) sobre la esquina superior
derecha de la pantalla, fuera del framebuffer manual. Cada celda del mapa
se pinta como un cuadrito pequeño con el color correspondiente a su tipo de
pared, y la posición/orientación del jugador se marca con un punto rojo y
una línea indicando hacia dónde mira.

---

## 9. Máquina de estados

El juego usa tres estados (`GameState::Welcome`, `Playing`, `Success`)
controlados con un `match` en el loop principal. Cada estado:
- Procesa su propio input (selección de nivel en Welcome, movimiento/disparo
  en Playing, volver al menú en Success).
- Dibuja su propia pantalla.
- Decide cuándo transicionar al siguiente estado (ENTER para jugar, todos
  los objetivos eliminados para pasar a Success).

La pantalla de éxito dibuja una puerta usando rectángulos superpuestos con
transparencia como se explico antes.

---

## 10. Música de fondo 

Se carga la música con `RaylibAudio::new_music`, la reproduce con
`play_stream()` una sola vez al inicio, y llama `update_stream()` en cada
frame del loop 

## Audio en WSL

Al correrlo en WSL y no escuchas la música/efectos de sonido,
es porque ALSA (usado por raylib) no encuentra tarjeta de audio física en
WSL y no enruta automáticamente por PulseAudio. Para arreglarlo:

\`\`\`bash
sudo apt install alsa-utils pulseaudio-utils libasound2-plugins -y
cat > ~/.asoundrc << 'EOF'
pcm.!default {
    type pulse
}
ctl.!default {
    type pulse
}
EOF
\`\`\`


---

## Controles

| Tecla / acción        | Función                          |
|------------------------|-----------------------------------|
| `W` `A` `S` `D`        | Moverse                          |
| Mouse                  | Rotar cámara (horizontal)         |
| Click izquierdo        | Disparar                         |
| `1` / `2` o flechas    | Elegir nivel (pantalla de inicio) |
| `ENTER`                 | Confirmar / continuar             |
| `ESC`                   | Volver al menú                    |

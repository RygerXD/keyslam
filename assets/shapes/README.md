# Shape SVG layers

Each active shape SVG is rendered from these named layers:

- `shadow`: shadow artwork, color, opacity, and offset.
- `outline`: outline geometry and width. KeySlam tints this layer black, or white for black shapes.
- `fill`: a white silhouette that KeySlam tints with the current item color.
- `shading`: highlight and shade artwork rendered exactly as authored.
- `face-placement`: a nearly transparent guide rectangle controlling the shared face's position and size. Change its `x`, `y`, `width`, and `height` values to reposition or resize the face for that shape.

`face.svg` remains the single source for the `smile`, `eyes-open`, and `eyes-closed` artwork. Restart KeySlam after editing an SVG because the assets are loaded when the app starts.

## Editing in GodSVG (recommended)

GodSVG works directly with standard SVG markup, so the names above remain ordinary `id` attributes. Keep those IDs unchanged when editing or exporting; KeySlam uses them to find each layer. GodSVG may warn that the `<defs>` container is unrecognized, but it preserves the container and its contents. Do not delete `silhouette`, `shape-shading`, or the `<defs>` container: the visible layers reference those definitions.

The closed-eye group in `face.svg` uses opacity `.001` so an editor displays the normal open face without drawing both eye states at once. Temporarily raise `eyes-closed` to opacity `1` while editing the blink, then restore it to `.001` before exporting. KeySlam restores the blink to full opacity when loading it.

## Editing in Graphite

Graphite displays SVG group IDs as generic `Untitled Layer` names. For shape files, its layer panel lists these from top to bottom as `face-placement`, `shading`, `fill`, `outline`, and `shadow`. The nearly transparent face-placement rectangle can be selected and resized on the canvas.

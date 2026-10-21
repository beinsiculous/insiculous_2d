// The conductor example's scripted trigger: Left Shift (player one's action 2) plays the
// next melody note on the piano for this entity's own source, beside the direct keys.
// @param binding: str = "melody-piano"

fn update(me, view, params, cmd, dt) {
    if view.just_activated(0, "action2") {
        cmd.trigger(me, params.binding);
    }
}

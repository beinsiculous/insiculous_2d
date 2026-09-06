// @param player: i32 = 0
// @param x: f32 = -370.0
// @param speed: f32 = 450.0
// @param ai: bool = false
// @param ai_speed: f32 = 255.0
// @param dead_zone: f32 = 2.0
// @param target: entity = "Ball"

fn early_update(me, view, params, cmd, dt) {
    let y = me.position.y;
    let new_y = y;
    if params.ai {
        let ball_pos = view.position(params.target);
        let diff = ball_pos.y - y;
        if diff.abs() > params.dead_zone {
            let dy = diff.sign() * params.ai_speed;
            new_y = y + dy * dt;
        }
    } else {
        let axis = view.move_y(params.player);
        let dy = axis * params.speed;
        new_y = y + dy * dt;
    }

    new_y = max(min(new_y, 230.0), -230.0);
    cmd.set_kinematic_target(me, vec2(params.x, new_y));
}

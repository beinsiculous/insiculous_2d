// @param speed: f32 = 250.0
// @param max_vertical: f32 = 500.0

fn early_update(me, view, params, cmd, dt) {
    let serving = view.blackboard_bool("serving", true);
    let game_over = view.blackboard_bool("game_over", false);

    if (view.just_activated(0, "action1") || view.just_activated(1, "action1")) && serving && !game_over {
        let last_scorer = view.blackboard_str("last_scorer", "right");
        let dir_x = if last_scorer == "left" { -1.0 } else { 1.0 };
        let hash = (view.frame * 2654435761) % 4294967296;
        let t = (hash >> 16).to_float() / 65535.0;
        let dir = vec2(dir_x, t * 1.2 - 0.6).normalize();
        cmd.set_velocity(me, dir * params.speed);
        cmd.set_blackboard_bool("serving", false);
    }
}

fn update(me, view, params, cmd, dt) {
    let serving = view.blackboard_bool("serving", true);
    if serving {
        return;
    }

    let vel = me.velocity;
    if vel.x.abs() < 0.1 {
        return;
    }

    let fixed_vx = vel.x.sign() * params.speed;
    let vy = max(min(vel.y, params.max_vertical), -params.max_vertical);
    let new_vel = vec2(fixed_vx, vy);

    if (new_vel - vel).length() > 1.0 {
        cmd.set_velocity(me, new_vel);
    }
}

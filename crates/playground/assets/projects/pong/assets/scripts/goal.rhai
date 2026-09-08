// @param side: str = "left"

fn update(me, view, params, cmd, dt) {
    if view.has_collision_started(me, "Ball") {
        if params.side == "left" {
            let right_score = view.blackboard_int("right_score", 0) + 1;
            cmd.set_blackboard_int("right_score", right_score);
            cmd.set_blackboard_str("last_scorer", "right");
        } else {
            let left_score = view.blackboard_int("left_score", 0) + 1;
            cmd.set_blackboard_int("left_score", left_score);
            cmd.set_blackboard_str("last_scorer", "left");
        }
        cmd.reset_body("Ball", vec2(0.0, 0.0));
        cmd.set_blackboard_bool("serving", true);
    }
}

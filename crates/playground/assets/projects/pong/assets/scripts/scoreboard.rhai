fn update(me, view, params, cmd, dt) {
    let left = view.blackboard_int("left_score", 0);
    let right = view.blackboard_int("right_score", 0);
    let game_over = view.blackboard_bool("game_over", false);

    if game_over {
        if view.just_activated(0, "action1") || view.just_activated(1, "action1") {
            cmd.set_blackboard_int("left_score", 0);
            cmd.set_blackboard_int("right_score", 0);
            cmd.set_blackboard_bool("game_over", false);
            cmd.set_blackboard_bool("serving", true);
            cmd.set_blackboard_str("last_scorer", "right");
            cmd.set_label_text(me, "0 : 0");
        }
        return;
    }

    if left >= 7 {
        cmd.set_label_text(me, "LEFT WINS — Action1 to restart");
        cmd.set_blackboard_bool("game_over", true);
    } else if right >= 7 {
        cmd.set_label_text(me, "RIGHT WINS — Action1 to restart");
        cmd.set_blackboard_bool("game_over", true);
    } else {
        cmd.set_label_text(me, `${left} : ${right}`);
    }
}

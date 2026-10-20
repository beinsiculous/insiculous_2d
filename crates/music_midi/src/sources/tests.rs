use super::*;

fn source(id: u64, generation: u64, epoch: u32) -> SourceId {
    SourceId {
        id,
        generation,
        epoch,
    }
}

#[test]
fn a_first_trigger_activates_a_source_and_its_deactivation_ends_that_activation_for_good() {
    let mut table = SourceTable::with_capacity(8);
    let slime = source(1, 0, 1);

    assert_eq!(table.admit(slime), Admission::Live { ended: None });
    assert_eq!(table.admit(slime), Admission::Live { ended: None });
    assert!(table.is_live(slime));
    assert_eq!(
        table.deactivate(slime),
        Deactivation::Ended {
            named: Some(slime),
            older: None
        }
    );

    assert_eq!(table.admit(slime), Admission::Dead);
    assert_eq!(table.deactivate(slime), Deactivation::Dead);
    assert_eq!(table.live_count(), 0);
}

#[test]
fn a_recycled_id_supersedes_its_old_generation_which_never_revives() {
    let mut table = SourceTable::with_capacity(8);
    let first_life = source(7, 0, 1);
    let recycled = source(7, 1, 2);
    table.admit(first_life);

    assert_eq!(
        table.admit(recycled),
        Admission::Live {
            ended: Some(first_life)
        },
        "the old generation's notes are the caller's to release"
    );
    assert_eq!(table.admit(first_life), Admission::Dead);
    assert_eq!(table.deactivate(first_life), Deactivation::Dead);
    assert!(table.is_live(recycled));
}

#[test]
fn a_reentered_source_is_a_new_epoch_and_the_old_epoch_stays_dead() {
    let mut table = SourceTable::with_capacity(8);
    let on_screen = source(3, 0, 4);
    let back_on_screen = source(3, 0, 9);
    table.admit(on_screen);
    table.deactivate(on_screen);

    assert_eq!(table.admit(back_on_screen), Admission::Live { ended: None });
    assert_eq!(table.admit(on_screen), Admission::Dead);
}

#[test]
fn a_late_deactivation_of_an_older_epoch_never_touches_the_newer_one() {
    let mut table = SourceTable::with_capacity(8);
    let left = source(3, 0, 4);
    let returned = source(3, 0, 5);
    table.admit(left);
    table.admit(returned);

    assert_eq!(table.deactivate(left), Deactivation::Dead);
    assert!(
        table.is_live(returned),
        "a deactivation retried after the re-entry names only its own epoch"
    );
}

#[test]
fn deactivating_a_newer_epoch_than_any_triggered_ends_the_older_one_and_remembers_the_new() {
    let mut table = SourceTable::with_capacity(8);
    let triggered = source(5, 0, 2);
    let never_triggered = source(5, 0, 6);
    table.admit(triggered);

    assert_eq!(
        table.deactivate(never_triggered),
        Deactivation::Ended {
            named: None,
            older: Some(triggered)
        }
    );
    assert_eq!(table.admit(never_triggered), Admission::Dead);
    assert_eq!(table.admit(triggered), Admission::Dead);
}

#[test]
fn a_flush_ends_only_sources_activated_before_its_epoch_and_refuses_those_epochs_after() {
    let mut table = SourceTable::with_capacity(8);
    let old_world = [source(1, 0, 1), source(2, 0, 2), source(3, 0, 3)];
    let new_world = source(4, 0, 10);
    for activation in old_world {
        table.admit(activation);
    }
    table.admit(new_world);
    let mut ended = Vec::new();

    table.flush(10, |source| ended.push(source));

    assert_eq!(ended, old_world.to_vec());
    assert!(
        table.is_live(new_world),
        "activated at the flush epoch: untouched"
    );
    assert_eq!(
        table.admit(source(99, 0, 9)),
        Admission::Dead,
        "an epoch below the floor is refused even for an id never seen"
    );
    assert_eq!(
        table.admit(source(1, 0, 11)),
        Admission::Live { ended: None }
    );
}

#[test]
fn a_full_table_forgets_a_dead_identity_before_it_evicts_the_oldest_live_source() {
    let mut table = SourceTable::with_capacity(2);
    let oldest = source(1, 0, 1);
    let dead = source(2, 0, 2);
    table.admit(oldest);
    table.admit(dead);
    table.deactivate(dead);

    assert_eq!(
        table.admit(source(3, 0, 3)),
        Admission::Live { ended: None }
    );
    assert_eq!(
        table.admit(source(4, 0, 4)),
        Admission::Live {
            ended: Some(oldest)
        },
        "with no dead entry left, the oldest live source makes room and is released"
    );
    assert_eq!(table.live_count(), 2);
}

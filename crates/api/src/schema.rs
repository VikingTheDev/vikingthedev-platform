// Hand-written to match `migrations/` (step 6 gate, 2026-10-19); the users tests read every column back.
diesel::table! {
    users (id) {
        id -> Uuid,
        display_name -> Text,
        created_at -> Timestamptz,
    }
}

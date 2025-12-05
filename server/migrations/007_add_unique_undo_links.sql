CREATE UNIQUE INDEX IF NOT EXISTS action_links_effect_undo_unique
  ON action_links(effect_action_id)
  WHERE relation_type = 'undo_of';

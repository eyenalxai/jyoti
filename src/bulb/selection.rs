use crate::bulb::model::Bulb;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    All,
    Bulb(String),
}

pub fn resolve_selection(bulbs: &[Bulb], previous: Option<&Selection>) -> Option<Selection> {
    if bulbs.len() >= 2 && (previous.is_none() || matches!(previous, Some(Selection::All))) {
        return Some(Selection::All);
    }
    let kept = match previous {
        Some(Selection::Bulb(id)) => bulbs.iter().find(|bulb| &bulb.id == id),
        _ => None,
    };
    let fallback = kept.or_else(|| bulbs.first())?;
    Some(Selection::Bulb(fallback.id.clone()))
}

pub fn selection_after_add(before: usize, id: &str, current: Option<&Selection>) -> Selection {
    if before == 0 {
        return Selection::Bulb(id.to_owned());
    }
    if before == 1 {
        return Selection::All;
    }
    current
        .cloned()
        .unwrap_or_else(|| Selection::Bulb(id.to_owned()))
}

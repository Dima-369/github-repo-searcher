/// Filter list by query case insensitively.
pub fn filter_human<T, F>(items: &[T], query: &str, mapper: F) -> Vec<T>
where
    T: Clone,
    F: Fn(&T) -> String,
{
    if items.is_empty() {
        return Vec::new();
    }

    let trimmed = query.trim();
    if trimmed.is_empty() {
        return items.to_vec();
    }

    let mut result = Vec::new();
    let query_parts: Vec<String> = trimmed
        .to_lowercase()
        .split(' ')
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect();

    // Sort query parts to handle exclusions first
    let query_parts = {
        let mut parts = query_parts;
        parts.sort_by(|a, b| {
            if a.starts_with('-') && !b.starts_with('-') {
                std::cmp::Ordering::Less
            } else if !a.starts_with('-') && b.starts_with('-') {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        parts
    };

    for item in items {
        let mapped = mapper(item).to_lowercase();
        let mut pass = true;

        for query_part in &query_parts {
            // Check length, so a single minus is still matched
            if query_part.len() >= 2 && query_part.starts_with('-') {
                if mapped.contains(&query_part[1..]) {
                    pass = false;
                    break;
                }
            } else if !mapped.contains(query_part) {
                pass = false;
                break;
            }
        }

        if pass {
            result.push(item.clone());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_list() {
        let items: Vec<String> = vec![];
        let result = filter_human(&items, "query", |s| s.clone());
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_empty_query() {
        let items = vec!["apple", "banana", "cherry"];
        let result = filter_human(&items, "", |s| s.to_string());
        assert_eq!(result, items);
    }

    #[test]
    fn test_simple_filter() {
        let items = vec!["apple", "banana", "cherry"];
        let result = filter_human(&items, "a", |s| s.to_string());
        assert_eq!(result, vec!["apple", "banana"]);
    }

    #[test]
    fn test_multiple_terms() {
        let items = vec!["apple pie", "banana split", "cherry pie"];
        let result = filter_human(&items, "pie a", |s| s.to_string());
        assert_eq!(result, vec!["apple pie"]);
    }

    #[test]
    fn test_exclusion() {
        let items = vec!["apple pie", "banana split", "cherry pie"];
        let result = filter_human(&items, "pie -cherry", |s| s.to_string());
        assert_eq!(result, vec!["apple pie"]);
    }

    #[test]
    fn test_case_insensitive() {
        let items = vec!["Apple", "Banana", "Cherry"];
        let result = filter_human(&items, "apple", |s| s.to_string());
        assert_eq!(result, vec!["Apple"]);
    }

    #[test]
    fn test_medical_medium_exclusion() {
        let items = vec![
            "medicalmedium-instagram (git@github.com:Dima-369/medicalmedium-instagram.git)",
            "medical-medium-text-files (git@github.com:Dima-369/medical-medium-text-files.git)",
            "medical-medium-demon-podcast-notes (git@github.com:Dima-369/medical-medium-demon-podcast-notes.git)"
        ];

        // Test with "medical -demon" to exclude demon podcast notes
        let result = filter_human(&items, "medical -demon", |s| s.to_string());
        assert_eq!(result, vec![
            "medicalmedium-instagram (git@github.com:Dima-369/medicalmedium-instagram.git)",
            "medical-medium-text-files (git@github.com:Dima-369/medical-medium-text-files.git)"
        ]);
    }
}

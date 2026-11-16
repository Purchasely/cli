pub fn suggest(input: &str, choices: &[&str]) -> Option<String> {
    let mut ranked = choices
        .iter()
        .map(|c| (distance(input, c), *c))
        .collect::<Vec<_>>();
    ranked.sort_by_key(|x| x.0);
    ranked
        .first()
        .filter(|(d, c)| *d <= 3 && *d * 2 <= input.chars().count().max(c.chars().count()))
        .map(|(_, c)| (*c).to_owned())
}

fn distance(a: &str, b: &str) -> usize {
    let a = a.chars().collect::<Vec<_>>();
    let b = b.chars().collect::<Vec<_>>();
    let mut d = vec![vec![0; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

#[cfg(test)]
mod tests {
    use super::suggest;
    #[test]
    fn did_you_mean() {
        assert_eq!(
            suggest("plaecments", &["placements", "products"]),
            Some("placements".into())
        );
    }
}

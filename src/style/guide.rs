//! Rendering the measured style into prompt text.

use super::StyleProfile;

impl StyleProfile {
    /// The style block handed to the model. Every line is a measurement, not an
    /// instruction, so the model imitates what it sees rather than a template.
    pub fn guide(&self) -> String {
        let pct = |value: f32| (value * 100.0).round() as u32;
        let mut out = String::new();

        out.push_str(&format!(
            "Measured from the last {} commits in this repo:\n",
            self.sample
        ));
        out.push_str(&format!(
            "- typical length {} chars (longest seen {})\n",
            self.avg_len, self.max_len
        ));
        out.push_str(&format!(
            "- first letter capitalised in {}% of them\n",
            pct(self.sentence_case_ratio)
        ));
        out.push_str(&format!(
            "- written as an imperative in {}% (verb first: \"Refactor X\", not \"Fixed X\" or \"Fixing X\")\n",
            100 - pct(self.non_imperative_ratio)
        ));
        out.push_str(&format!(
            "- `type:` prefixes (feat/fix/chore/refactor) appear in {}%\n",
            pct(self.conventional_ratio)
        ));
        out.push_str(&format!(
            "- end with a full stop: {}%\n",
            pct(self.trailing_period_ratio)
        ));
        out.push_str(&format!(
            "- mention a ticket id: {}%\n",
            pct(self.ticket_ratio)
        ));
        if self.emoji_count > 0 {
            out.push_str(&format!(
                "- emoji appear in {} messages\n",
                self.emoji_count
            ));
        }
        out.push_str(&format!("- language: {}\n", self.language.label()));
        if !self.top_verbs.is_empty() {
            let verbs: Vec<String> = self
                .top_verbs
                .iter()
                .map(|(verb, count)| format!("{verb} ({count})"))
                .collect();
            out.push_str(&format!(
                "- opening verbs this author reaches for: {}\n",
                verbs.join(", ")
            ));
        }
        out
    }

    /// One line for `-v`.
    pub fn summary_line(&self) -> String {
        let window = if self.newest_sha.is_empty() {
            "no history".to_string()
        } else {
            format!("newest {}", self.newest_sha)
        };
        format!(
            "{} commits sampled ({window}), {:.0}% templated -> {} style, {}",
            self.sample,
            self.conventional_ratio * 100.0,
            self.style_label(),
            self.language.label()
        )
    }
}

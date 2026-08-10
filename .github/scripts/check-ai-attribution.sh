#!/usr/bin/env bash
set -euo pipefail

base_revision="${1:?base revision is required}"
head_revision="${2:?head revision is required}"

git rev-parse --verify "${base_revision}^{commit}" >/dev/null
git rev-parse --verify "${head_revision}^{commit}" >/dev/null

violations="$({
    git log -z --format='%H%x00%B' "${base_revision}..${head_revision}" |
        perl -0 -ne '
            s/\0\z//;
            if (!defined $commit) {
                $commit = $_;
                next;
            }
            while (/^(Co-authored-by:\s*([^\r\n<]*?)\s*<([^>\r\n]+)>)[\t ]*$/gmi) {
                my ($trailer, $name, $email) = ($1, $2, $3);
                my $identity = "$name $email";
                my $known_tool = qr/\b(?:claude(?:\s+code)?|codex|chatgpt|droid|factory\s+droid|github\s+copilot|copilot|gemini|cursor|windsurf|devin|aider|cline|cody|qodo|codeium|tabnine|junie|augment|roo\s+code|amazon\s+q|replit\s+agent|amp)\b/i;
                my $known_address = qr/(?:noreply\@anthropic\.com|codex\@openai\.com|droid\@factory\.ai|\[bot\])/i;
                print "$commit\t$trailer\n" if $identity =~ $known_tool || $identity =~ $known_address;
            }
            undef $commit;
        '
})"

if [[ -z "$violations" ]]; then
    exit 0
fi

printf '%s\n' 'BoltFFI does not accept commits attributed to AI tools.' >&2
printf '%s\n' 'AI-assisted work is welcome, but AI is a tool rather than a substitute for human ownership.' >&2
printf '%s\n' 'Contributors must understand and verify the change and take responsibility for it.' >&2
printf '%s\n' 'Remove the AI Co-authored-by trailer and amend each listed commit:' >&2
printf '%s\n' "$violations" >&2
exit 1

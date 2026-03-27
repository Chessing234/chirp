import { useMemo } from 'react';

function fuzzyMatch(pattern: string, text: string): { score: number; matches: number[] } {
  const patternLower = pattern.toLowerCase();
  const textLower = text.toLowerCase();

  if (!pattern) return { score: 1, matches: [] };

  let patternIdx = 0;
  let score = 0;
  let consecutiveBonus = 0;
  const matches: number[] = [];

  for (let textIdx = 0; textIdx < textLower.length && patternIdx < patternLower.length; textIdx++) {
    if (textLower[textIdx] === patternLower[patternIdx]) {
      matches.push(textIdx);
      score += 1 + consecutiveBonus;

      // Bonus for word boundaries
      if (textIdx === 0 || /\s/.test(text[textIdx - 1])) {
        score += 5;
      }

      consecutiveBonus = Math.min(consecutiveBonus + 1, 5);
      patternIdx++;
    } else {
      consecutiveBonus = 0;
    }
  }

  // Return 0 if not all pattern chars matched
  if (patternIdx !== patternLower.length) {
    return { score: 0, matches: [] };
  }

  // Normalize score by text length
  return { score: score / text.length, matches };
}

export function useFuzzySearch<T extends { content: string }>(
  items: T[],
  query: string
): Array<T & { score: number; matches: number[] }> {
  return useMemo(() => {
    if (!query.trim()) {
      return items.map((item) => ({ ...item, score: 1, matches: [] }));
    }

    const results = items
      .map((item) => {
        const { score, matches } = fuzzyMatch(query, item.content);
        return { ...item, score, matches };
      })
      .filter((item) => item.score > 0)
      .sort((a, b) => b.score - a.score);

    return results;
  }, [items, query]);
}

export function highlightMatches(text: string, matches: number[]): string {
  if (matches.length === 0) return text;

  let result = '';
  let lastIndex = 0;

  for (const idx of matches) {
    result += text.slice(lastIndex, idx);
    result += `<mark class="bg-ping-accent/30 text-ping-text">${text[idx]}</mark>`;
    lastIndex = idx + 1;
  }

  result += text.slice(lastIndex);
  return result;
}

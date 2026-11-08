export interface FormatOptions {
  thousands: boolean;
  decimalSeparator: string;
  groupSeparator: string;
  /** Integers with more digits than this are shown in scientific notation. */
  maxIntegerDigits: number;
}

export const DEFAULT_FORMAT: FormatOptions = {
  thousands: false,
  decimalSeparator: ".",
  groupSeparator: ",",
  maxIntegerDigits: 21,
};

const DECIMAL = /^(-?)(\d+)(?:\.(\d+))?$/;

export function groupDigits(digits: string, separator: string): string {
  const groups: string[] = [];
  for (let end = digits.length; end > 0; end -= 3) {
    groups.unshift(digits.slice(Math.max(0, end - 3), end));
  }
  return groups.join(separator);
}

function scientific(sign: string, integer: string, fraction: string): string {
  const digits = (integer + fraction).replace(/0+$/, "");
  const mantissa = digits.length > 1 ? `${digits[0]}.${digits.slice(1, 7)}` : digits;
  return `${sign}${mantissa}e+${integer.length - 1}`;
}

/** Formats the decimal text produced by the engine (`-1234.5`) for display. */
export function formatNumber(text: string, overrides: Partial<FormatOptions> = {}): string {
  const options = { ...DEFAULT_FORMAT, ...overrides };
  const match = DECIMAL.exec(text);
  if (!match) return text;
  const [, sign, integer, fraction = ""] = match;
  if (integer.length > options.maxIntegerDigits) return scientific(sign, integer, fraction);
  const grouped = options.thousands ? groupDigits(integer, options.groupSeparator) : integer;
  return fraction ? `${sign}${grouped}${options.decimalSeparator}${fraction}` : `${sign}${grouped}`;
}

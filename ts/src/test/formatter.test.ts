import assert from "node:assert/strict";
import { test } from "node:test";
import { formatNumber, groupDigits } from "../formatter.js";

test("plain numbers pass through", () => {
  assert.equal(formatNumber("42"), "42");
  assert.equal(formatNumber("-0.5"), "-0.5");
  assert.equal(formatNumber("1234567.891"), "1234567.891");
});

test("thousands grouping", () => {
  assert.equal(formatNumber("1234567", { thousands: true }), "1,234,567");
  assert.equal(formatNumber("999", { thousands: true }), "999");
  assert.equal(formatNumber("1000", { thousands: true }), "1,000");
  assert.equal(formatNumber("100000", { thousands: true }), "100,000");
  assert.equal(formatNumber("-1234567.25", { thousands: true }), "-1,234,567.25");
  assert.equal(formatNumber("0.123456", { thousands: true }), "0.123456");
});

test("custom separators", () => {
  const options = { thousands: true, decimalSeparator: ",", groupSeparator: "." };
  assert.equal(formatNumber("1234567.89", options), "1.234.567,89");
  assert.equal(formatNumber("0.5", { decimalSeparator: "," }), "0,5");
});

test("very large integers use scientific notation", () => {
  assert.equal(formatNumber("1" + "0".repeat(24)), "1e+24");
  assert.equal(formatNumber("-1234567" + "0".repeat(20)), "-1.234567e+26");
  assert.equal(formatNumber("9".repeat(21)), "9".repeat(21));
});

test("text that is not a plain decimal is left alone", () => {
  assert.equal(formatNumber("1/3"), "1/3");
  assert.equal(formatNumber("abc"), "abc");
  assert.equal(formatNumber(""), "");
});

test("groupDigits handles short and exact multiples", () => {
  assert.equal(groupDigits("1", ","), "1");
  assert.equal(groupDigits("123456", ","), "123,456");
  assert.equal(groupDigits("1234", " "), "1 234");
});

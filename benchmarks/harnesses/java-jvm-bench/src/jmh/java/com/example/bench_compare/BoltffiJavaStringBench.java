package com.example.bench_compare;

import com.example.bench_boltffi.BenchBoltFFI;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.Benchmark;
import org.openjdk.jmh.annotations.BenchmarkMode;
import org.openjdk.jmh.annotations.Level;
import org.openjdk.jmh.annotations.Mode;
import org.openjdk.jmh.annotations.OutputTimeUnit;
import org.openjdk.jmh.annotations.Scope;
import org.openjdk.jmh.annotations.Setup;
import org.openjdk.jmh.annotations.State;
import org.openjdk.jmh.infra.Blackhole;

@BenchmarkMode(Mode.AverageTime)
@OutputTimeUnit(TimeUnit.NANOSECONDS)
@State(Scope.Thread)
public class BoltffiJavaStringBench {
    private String small;
    private String ascii200;
    private String ascii1k;
    private String ascii64k;
    private String unicode;

    @Setup(Level.Trial)
    public void prepareStrings() {
        small = "hello";
        ascii200 = "x".repeat(200);
        ascii1k = "x".repeat(1_000);
        ascii64k = "x".repeat(65_536);
        unicode = "BoltFFI 🦀 Ελληνικά 日本語 مرحبا";
        verifyBehavior();
    }

    @Benchmark
    public void boltffi_java_echo_string_small(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoString(small));
    }

    @Benchmark
    public void boltffi_java_echo_string_200(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoString(ascii200));
    }

    @Benchmark
    public void boltffi_java_echo_string_1k(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoString(ascii1k));
    }

    @Benchmark
    public void boltffi_java_echo_string_64k(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoString(ascii64k));
    }

    @Benchmark
    public void boltffi_java_echo_string_unicode(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoString(unicode));
    }

    @Benchmark
    public void boltffi_java_concat_strings(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.concatStrings(ascii200, unicode));
    }

    @Benchmark
    public void boltffi_java_string_length_1k(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.stringLength(ascii1k));
    }

    @Benchmark
    public void boltffi_java_string_is_empty(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.stringIsEmpty(""));
    }

    @Benchmark
    public void boltffi_java_repeat_string(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.repeatString(small, 20));
    }

    @Benchmark
    public void boltffi_java_generate_string_1k(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.generateString(1_000));
    }

    @Benchmark
    public void boltffi_java_generate_string_64k(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.generateString(65_536));
    }

    private void verifyBehavior() {
        requireEqual(small, BenchBoltFFI.echoString(small), "small string");
        requireEqual(ascii200, BenchBoltFFI.echoString(ascii200), "200-byte string");
        requireEqual(ascii1k, BenchBoltFFI.echoString(ascii1k), "1 KiB string");
        requireEqual(ascii64k, BenchBoltFFI.echoString(ascii64k), "64 KiB string");
        requireEqual(unicode, BenchBoltFFI.echoString(unicode), "Unicode string");
        requireEqual(ascii200 + unicode, BenchBoltFFI.concatStrings(ascii200, unicode), "concatenation");
        requireEqual(small.repeat(20), BenchBoltFFI.repeatString(small, 20), "repetition");
        requireEqual("x".repeat(1_000), BenchBoltFFI.generateString(1_000), "generated 1 KiB string");
        requireEqual("x".repeat(65_536), BenchBoltFFI.generateString(65_536), "generated 64 KiB string");
        if (BenchBoltFFI.stringLength(unicode) != unicode.getBytes(StandardCharsets.UTF_8).length) {
            throw new AssertionError("string length behavior mismatch");
        }
        if (!BenchBoltFFI.stringIsEmpty("") || BenchBoltFFI.stringIsEmpty(small)) {
            throw new AssertionError("string emptiness behavior mismatch");
        }
        String malformed = new String(new char[] {'\uD800', 'x', '\uDC00'});
        String replaced = new String(malformed.getBytes(StandardCharsets.UTF_8), StandardCharsets.UTF_8);
        requireEqual(replaced, BenchBoltFFI.echoString(malformed), "malformed UTF-16 replacement");
    }

    private void requireEqual(String expected, String actual, String operation) {
        if (!expected.equals(actual)) {
            throw new AssertionError(operation + " behavior mismatch");
        }
    }
}

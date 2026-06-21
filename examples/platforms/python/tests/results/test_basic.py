import math

from tests.support import DemoTestCase

import demo


class BasicResultTests(DemoTestCase):
    def assert_runtime_error_contains(self, expected: str, call) -> None:
        with self.assertRaises(RuntimeError) as error:
            call()
        self.assertIn(expected, str(error.exception))

    def test_string_error_results(self) -> None:
        self.demo_case("case:results.basic.safe_divide.should_return_quotient")
        self.assertEqual(demo.safe_divide(10, 2), 5)
        self.demo_case("case:results.basic.safe_divide.should_reject_division_by_zero")
        self.assert_runtime_error_contains("division by zero", lambda: demo.safe_divide(1, 0))
        self.demo_case("case:results.basic.safe_sqrt.should_return_square_root")
        self.assertTrue(math.isclose(demo.safe_sqrt(9.0), 3.0, rel_tol=0.0, abs_tol=1e-12))
        self.demo_case("case:results.basic.safe_sqrt.should_reject_negative_input")
        self.assert_runtime_error_contains("negative input", lambda: demo.safe_sqrt(-1.0))
        self.demo_case("case:results.basic.parse_point.should_parse_coordinates")
        self.assertEqual(demo.parse_point("1.5, 2.5"), demo.Point(1.5, 2.5))
        self.demo_case("case:results.basic.parse_point.should_reject_malformed_input")
        self.assert_runtime_error_contains("expected format", lambda: demo.parse_point("wat"))
        self.demo_case("case:results.basic.always_ok.should_return_doubled_value")
        self.assertEqual(demo.always_ok(21), 42)
        self.demo_case("case:results.basic.always_err.should_return_message_error")
        self.assert_runtime_error_contains("boom", lambda: demo.always_err("boom"))
        self.demo_case("case:results.basic.divide.should_return_quotient")
        self.assertEqual(demo.divide(10, 2), 5)
        self.demo_case("case:results.basic.divide.should_reject_division_by_zero")
        self.assert_runtime_error_contains("division by zero", lambda: demo.divide(10, 0))
        self.demo_case("case:results.basic.parse_int.should_parse_integer")
        self.assertEqual(demo.parse_int("42"), 42)
        self.demo_case("case:results.basic.parse_int.should_reject_invalid_integer")
        self.assert_runtime_error_contains("invalid integer", lambda: demo.parse_int("nope"))
        self.demo_case("case:results.basic.validate_name.should_greet_valid_name")
        self.assertEqual(demo.validate_name("Ali"), "Hello, Ali!")
        self.demo_case("case:results.basic.validate_name.should_reject_empty_name")
        self.assert_runtime_error_contains("name cannot be empty", lambda: demo.validate_name(""))

    def test_result_parameter(self) -> None:
        self.demo_case("case:results.basic.result_to_string.should_render_ok")
        self.assertEqual(demo.result_to_string((True, 7)), "ok: 7")
        self.demo_case("case:results.basic.result_to_string.should_render_err")
        self.assertEqual(demo.result_to_string((False, "bad")), "err: bad")

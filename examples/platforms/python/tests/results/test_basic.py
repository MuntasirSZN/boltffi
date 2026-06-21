import math

from tests.support import DemoTestCase

import demo


class BasicResultTests(DemoTestCase):
    def test_string_error_results(self) -> None:
        self.demo_case("case:results.basic.safe_divide.should_return_quotient")
        self.assertEqual(demo.safe_divide(10, 2), 5)
        self.demo_case("case:results.basic.safe_sqrt.should_return_square_root")
        self.assertTrue(math.isclose(demo.safe_sqrt(9.0), 3.0, rel_tol=0.0, abs_tol=1e-12))
        self.demo_case("case:results.basic.parse_point.should_parse_coordinates")
        self.assertEqual(demo.parse_point("1.5, 2.5"), demo.Point(1.5, 2.5))
        self.demo_case("case:results.basic.always_ok.should_return_doubled_value")
        self.assertEqual(demo.always_ok(21), 42)
        self.demo_case("case:results.basic.divide.should_return_quotient")
        self.assertEqual(demo.divide(10, 2), 5)
        self.demo_case("case:results.basic.parse_int.should_parse_integer")
        self.assertEqual(demo.parse_int("42"), 42)
        self.demo_case("case:results.basic.validate_name.should_greet_valid_name")
        self.assertEqual(demo.validate_name("Ali"), "Hello, Ali!")

    def test_result_parameter(self) -> None:
        self.demo_case("case:results.basic.result_to_string.should_render_ok")
        self.assertEqual(demo.result_to_string((True, 7)), "ok: 7")
        self.demo_case("case:results.basic.result_to_string.should_render_err")
        self.assertEqual(demo.result_to_string((False, "bad")), "err: bad")

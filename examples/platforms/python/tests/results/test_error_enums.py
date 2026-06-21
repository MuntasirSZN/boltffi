from tests.support import DemoTestCase

import demo


class ErrorEnumResultTests(DemoTestCase):
    def assert_runtime_error(self, call) -> None:
        with self.assertRaises(RuntimeError):
            call()

    def test_success_paths(self) -> None:
        self.demo_case("case:results.error_enums.checked_divide.should_return_quotient")
        self.assertEqual(demo.checked_divide(10, 2), 5)
        self.demo_case("case:results.error_enums.checked_sqrt.should_return_square_root")
        self.assertEqual(demo.checked_sqrt(9.0), 3.0)
        self.demo_case("case:results.error_enums.checked_add.should_return_sum")
        self.assertEqual(demo.checked_add(2, 3), 5)
        self.demo_case("case:results.error_enums.validate_username.should_accept_valid_name")
        self.assertEqual(demo.validate_username("valid_name"), "valid_name")
        self.demo_case("case:results.error_enums.may_fail.should_return_success_when_valid")
        self.assertEqual(demo.may_fail(True), "Success!")
        self.demo_case("case:results.error_enums.divide_app.should_return_quotient")
        self.assertEqual(demo.divide_app(10, 2), 5)
        self.demo_case("case:results.error_enums.try_compute.should_return_doubled_value")
        self.assertEqual(demo.try_compute(3), 6)

    def test_data_enum_returns(self) -> None:
        self.demo_case("case:results.error_enums.process_value.should_return_success_variant")
        self.assertEqual(demo.process_value(3), demo.ApiResultSuccess())
        self.demo_case("case:results.error_enums.process_value.should_return_error_code_variant")
        self.assertEqual(demo.process_value(0), demo.ApiResultErrorCode(-1))
        self.demo_case("case:results.error_enums.process_value.should_return_error_with_data_variant")
        self.assertEqual(demo.process_value(-3), demo.ApiResultErrorWithData(-3, -6))
        self.demo_case("case:results.error_enums.api_result_is_success.should_report_success_variant")
        self.assertIs(demo.api_result_is_success(demo.ApiResultSuccess()), True)

    def test_success_response(self) -> None:
        point = demo.DataPoint(1.0, 2.0, 3)

        self.demo_case("case:results.error_enums.benchmark_response.should_make_success_response")
        self.assertEqual(demo.create_success_response(7, point), demo.BenchmarkResponse(7, (True, point)))
        success_envelope = demo.BenchmarkResponse(11, (True, demo.DataPoint(4.0, 5.0, 6)))
        self.demo_case("case:results.error_enums.benchmark_response.should_report_success_response")
        self.assertIs(demo.is_response_success(success_envelope), True)
        self.demo_case("case:results.error_enums.benchmark_response.should_return_value_for_success_response")
        self.assertEqual(demo.get_response_value(success_envelope), demo.DataPoint(4.0, 5.0, 6))

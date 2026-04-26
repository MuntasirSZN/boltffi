import unittest

import demo


class RecordsWithStringsTests(unittest.TestCase):
    def test_person_functions(self) -> None:
        person = demo.Person("Bob", 25)

        self.assertEqual(demo.echo_person(person), person)
        self.assertEqual(demo.make_person("Alice", 30), demo.Person("Alice", 30))
        self.assertEqual(
            demo.greet_person(demo.Person("Charlie", 40)),
            "Hello, Charlie! You are 40 years old.",
        )

    def test_person_utf8_round_trip(self) -> None:
        person = demo.Person("🎉 Party", 25)

        self.assertEqual(demo.echo_person(person), person)

    def test_address_functions(self) -> None:
        address = demo.Address("Main", "Amsterdam", "1000")

        self.assertEqual(demo.echo_address(address), address)
        self.assertEqual(demo.format_address(address), "Main, Amsterdam, 1000")

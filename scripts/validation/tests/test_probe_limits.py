import sys
import unittest
from datetime import date
from pathlib import Path

VALIDATION_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(VALIDATION_DIR))

import probe_limits  # noqa: E402


class FakeError(Exception):
    def __init__(self, message, status_code=None):
        super().__init__(message)
        self.status_code = status_code


class AcceptingClient:
    class Responses:
        @staticmethod
        def create(**_kwargs):
            return object()

    responses = Responses()


def ref_hops(schema):
    definitions = schema["$defs"]
    ref = schema["properties"]["value"]["$ref"]
    hops = 0
    seen = []
    while ref:
        hops += 1
        name = ref.removeprefix("#/$defs/")
        seen.append(name)
        ref = definitions[name].get("$ref")
    return hops, seen


class ProbeLimitTests(unittest.TestCase):
    def test_live_case_matrix_includes_required_reference_boundaries(self):
        groups = {section: cases for section, _title, cases in probe_limits.probe_groups()}
        self.assertEqual([case["hops"] for case, _schema in groups["ref_depth"]], [10, 11])
        self.assertEqual(
            [case["cycle_size"] for case, _schema in groups["ref_cycle"]],
            [1, 2],
        )

    def test_local_ref_chains_have_exact_hop_counts(self):
        for depth in (10, 11):
            schema = probe_limits.local_ref_chain(depth)
            hops, seen = ref_hops(schema)
            self.assertEqual(hops, depth)
            self.assertEqual(seen, [f"hop{index}" for index in range(1, depth + 1)])
            self.assertEqual(schema["$defs"][f"hop{depth}"], {"type": "string"})

    def test_local_ref_cycles_cover_self_and_mutual_cycles(self):
        self_cycle = probe_limits.local_ref_cycle(1)
        self.assertEqual(self_cycle["$defs"]["node1"]["$ref"], "#/$defs/node1")

        mutual_cycle = probe_limits.local_ref_cycle(2)
        self.assertEqual(mutual_cycle["$defs"]["node1"]["$ref"], "#/$defs/node2")
        self.assertEqual(mutual_cycle["$defs"]["node2"]["$ref"], "#/$defs/node1")

    def test_schema_rejections_are_provider_verdicts(self):
        outcome = probe_limits.classify_exception(
            FakeError("Invalid schema for response_format", 400)
        )
        self.assertEqual(outcome["kind"], "provider_verdict")
        self.assertEqual(outcome["status"], "rejected")

    def test_successful_submission_is_an_accepted_provider_verdict(self):
        outcome = probe_limits.submit(AcceptingClient(), {"type": "string"})
        self.assertEqual(outcome, {
            "kind": "provider_verdict",
            "status": "accepted",
            "error": None,
        })

    def test_auth_and_transport_errors_are_infrastructure(self):
        auth = probe_limits.classify_exception(FakeError("bad key", 401))
        transport = probe_limits.classify_exception(FakeError("connection reset"))
        self.assertEqual((auth["kind"], auth["category"]), (
            "infrastructure_failure",
            "authentication",
        ))
        self.assertEqual((transport["kind"], transport["category"]), (
            "infrastructure_failure",
            "transport",
        ))

    def test_infrastructure_failures_never_enter_verdict_results(self):
        results = {"ref_depth": []}
        infrastructure = []
        retained = probe_limits.record_outcome(
            results,
            infrastructure,
            "ref_depth",
            {"hops": 10},
            probe_limits.classify_exception(FakeError("bad key", 403)),
        )
        self.assertFalse(retained)
        self.assertEqual(results["ref_depth"], [])
        self.assertEqual(infrastructure[0]["category"], "authentication")

    def test_artifact_path_uses_requested_path_or_current_date(self):
        requested = Path("custom/evidence.json")
        self.assertEqual(probe_limits.artifact_path(requested), requested)
        self.assertEqual(
            probe_limits.artifact_path(today=date(2030, 1, 2)).name,
            "probe_limits_2030-01-02.json",
        )


if __name__ == "__main__":
    unittest.main()

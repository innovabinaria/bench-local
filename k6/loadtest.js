import http from "k6/http";
import { sleep } from "k6";

export const options = {
  scenarios: {
    ramp: {
      executor: "ramping-vus",
      startVUs: 10,
      stages: [
        { duration: "10s", target: 50 },
        { duration: "10s", target: 200 },
        { duration: "10s", target: 500 }
      ]
    }
  }
};

export default function () {
  http.get("http://host.docker.internal:8080/api/item/1");
  sleep(0.001);
}

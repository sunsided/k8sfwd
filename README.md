# Kubernetes Port Forwarding Automation

Provide a `.k8sfwd` file in your project root and
run this application to forward to multiple targets
at the same time.

```yaml
---
version: 0.1.0
targets:
  - name: Test API (Staging)
    target: foo
    type: service
    namespace: bar
    context: null
    cluster: null
    ports:
      - "5012:80"
      - "8080"
  - name: Test API (Production)
    target: foo-59b58f5d68-6t6bh
    type: pod
    namespace: bar
    cluster: production
    listen_addrs:
      - "127.1.0.1"
    ports:
      - "5012:80"
```

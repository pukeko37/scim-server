// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="introduction.html">Introduction</a></li><li class="chapter-item expanded affix "><li class="part-title">Getting Started</li><li class="chapter-item expanded "><a href="getting-started/installation.html"><strong aria-hidden="true">1.</strong> Installation</a></li><li class="chapter-item expanded "><a href="getting-started/first-server.html"><strong aria-hidden="true">2.</strong> Your First SCIM Server</a></li><li class="chapter-item expanded "><a href="getting-started/basic-operations.html"><strong aria-hidden="true">3.</strong> Basic Operations</a></li><li class="chapter-item expanded affix "><li class="part-title">Core Concepts</li><li class="chapter-item expanded "><a href="concepts/scim-protocol.html"><strong aria-hidden="true">4.</strong> SCIM Protocol Overview</a></li><li class="chapter-item expanded "><a href="concepts/architecture.html"><strong aria-hidden="true">5.</strong> Architecture</a></li><li class="chapter-item expanded "><a href="concepts/resource-model.html"><strong aria-hidden="true">6.</strong> Resource Model</a></li><li class="chapter-item expanded "><a href="concepts/multi-tenancy.html"><strong aria-hidden="true">7.</strong> Multi-Tenancy</a></li><li class="chapter-item expanded "><a href="concepts/providers.html"><strong aria-hidden="true">8.</strong> Storage Providers</a></li><li class="chapter-item expanded "><a href="concepts/etag-concurrency.html"><strong aria-hidden="true">9.</strong> ETag Concurrency Control</a></li><li class="chapter-item expanded affix "><li class="part-title">Tutorials</li><li class="chapter-item expanded "><a href="tutorials/custom-resources.html"><strong aria-hidden="true">10.</strong> Custom Resource Types</a></li><li class="chapter-item expanded "><a href="tutorials/authentication-setup.html"><strong aria-hidden="true">11.</strong> Authentication Setup</a></li><li class="chapter-item expanded "><a href="tutorials/multi-tenant-deployment.html"><strong aria-hidden="true">12.</strong> Multi-Tenant Deployment</a></li><li class="chapter-item expanded "><a href="tutorials/framework-integration.html"><strong aria-hidden="true">13.</strong> Web Framework Integration</a></li><li class="chapter-item expanded "><a href="tutorials/mcp-integration.html"><strong aria-hidden="true">14.</strong> AI Integration with MCP</a></li><li class="chapter-item expanded "><a href="tutorials/performance-optimization.html"><strong aria-hidden="true">15.</strong> Performance Optimization</a></li><li class="chapter-item expanded affix "><li class="part-title">Validation</li><li class="chapter-item expanded "><a href="validation/overview.html"><strong aria-hidden="true">16.</strong> Overview</a></li><li class="chapter-item expanded "><a href="validation/basic.html"><strong aria-hidden="true">17.</strong> Basic Validation</a></li><li class="chapter-item expanded "><a href="validation/advanced.html"><strong aria-hidden="true">18.</strong> Advanced Validation</a></li><li class="chapter-item expanded "><a href="validation/field-level.html"><strong aria-hidden="true">19.</strong> Field-Level Validation</a></li><li class="chapter-item expanded "><a href="validation/configuration.html"><strong aria-hidden="true">20.</strong> Configuration</a></li><li class="chapter-item expanded affix "><li class="part-title">Providers</li><li class="chapter-item expanded "><a href="providers/architecture.html"><strong aria-hidden="true">21.</strong> Architecture</a></li><li class="chapter-item expanded "><a href="providers/basic.html"><strong aria-hidden="true">22.</strong> Basic Implementation</a></li><li class="chapter-item expanded "><a href="providers/advanced.html"><strong aria-hidden="true">23.</strong> Advanced Features</a></li><li class="chapter-item expanded "><a href="providers/testing.html"><strong aria-hidden="true">24.</strong> Testing</a></li><li class="chapter-item expanded affix "><li class="part-title">Schemas</li><li class="chapter-item expanded "><a href="schemas/overview.html"><strong aria-hidden="true">25.</strong> Overview</a></li><li class="chapter-item expanded "><a href="schemas/custom-resources.html"><strong aria-hidden="true">26.</strong> Custom Resources</a></li><li class="chapter-item expanded "><a href="schemas/extensions.html"><strong aria-hidden="true">27.</strong> Extensions</a></li><li class="chapter-item expanded "><a href="schemas/validation.html"><strong aria-hidden="true">28.</strong> Validation</a></li><li class="chapter-item expanded affix "><li class="part-title">Concurrency</li><li class="chapter-item expanded "><a href="concurrency/overview.html"><strong aria-hidden="true">29.</strong> Overview</a></li><li class="chapter-item expanded "><a href="concurrency/implementation.html"><strong aria-hidden="true">30.</strong> Implementation</a></li><li class="chapter-item expanded "><a href="concurrency/conflict-resolution.html"><strong aria-hidden="true">31.</strong> Conflict Resolution</a></li><li class="chapter-item expanded affix "><li class="part-title">How-To Guides</li><li class="chapter-item expanded "><a href="how-to/migrate-versions.html"><strong aria-hidden="true">32.</strong> Migrate Between Versions</a></li><li class="chapter-item expanded "><a href="how-to/troubleshooting.html"><strong aria-hidden="true">33.</strong> Troubleshooting</a></li><li class="chapter-item expanded affix "><li class="part-title">Advanced Topics</li><li class="chapter-item expanded "><a href="advanced/production-deployment.html"><strong aria-hidden="true">34.</strong> Production Deployment</a></li><li class="chapter-item expanded "><a href="advanced/security.html"><strong aria-hidden="true">35.</strong> Security Considerations</a></li><li class="chapter-item expanded "><a href="advanced/monitoring.html"><strong aria-hidden="true">36.</strong> Monitoring and Observability</a></li><li class="chapter-item expanded affix "><li class="part-title">Reference</li><li class="chapter-item expanded "><a href="reference/api-endpoints.html"><strong aria-hidden="true">37.</strong> API Endpoints</a></li><li class="chapter-item expanded "><a href="reference/error-codes.html"><strong aria-hidden="true">38.</strong> Error Codes</a></li><li class="chapter-item expanded "><a href="reference/configuration.html"><strong aria-hidden="true">39.</strong> Configuration Options</a></li><li class="chapter-item expanded "><a href="reference/scim-compliance.html"><strong aria-hidden="true">40.</strong> SCIM Compliance</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);

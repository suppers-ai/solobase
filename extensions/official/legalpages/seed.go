package legalpages

import (
	"context"
	"database/sql"
	"fmt"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"

	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
)

// SeedDataWithSQL seeds initial legal documents for the legalpages extension using sqlc
func SeedDataWithSQL(sqlDB *sql.DB) error {
	ctx := context.Background()
	queries := db.New(sqlDB)

	// Check if data already exists
	count, err := queries.CountLegalDocumentsByType(ctx, "terms")
	if err != nil {
		return fmt.Errorf("failed to check existing data: %w", err)
	}
	if count > 0 {
		return nil // Data already seeded
	}

	// Create template Terms and Conditions
	termsContent := `<h2>1. Acceptance of Terms</h2>
<p>By accessing and using this service, you accept and agree to be bound by the terms and provision of this agreement.</p>

<h2>2. Use License</h2>
<p>Permission is granted to temporarily download one copy of the materials (information or software) on our service for personal, non-commercial transitory viewing only. This is the grant of a license, not a transfer of title, and under this license you may not:</p>
<ul>
<li>modify or copy the materials;</li>
<li>use the materials for any commercial purpose, or for any public display (commercial or non-commercial);</li>
<li>attempt to decompile or reverse engineer any software contained on our service;</li>
<li>remove any copyright or other proprietary notations from the materials.</li>
</ul>

<h2>3. Disclaimer</h2>
<p>The materials on our service are provided on an 'as is' basis. We make no warranties, expressed or implied, and hereby disclaim and negate all other warranties including, without limitation, implied warranties or conditions of merchantability, fitness for a particular purpose, or non-infringement of intellectual property or other violation of rights.</p>

<h2>4. Limitations</h2>
<p>In no event shall our organization or its suppliers be liable for any damages (including, without limitation, damages for loss of data or profit, or due to business interruption) arising out of the use or inability to use the materials on our service, even if we or our authorized representative has been notified orally or in writing of the possibility of such damage. Because some jurisdictions do not allow limitations on implied warranties, or limitations of liability for consequential or incidental damages, these limitations may not apply to you.</p>

<h2>5. Accuracy of Materials</h2>
<p>The materials appearing on our service could include technical, typographical, or photographic errors. We do not warrant that any of the materials on its service are accurate, complete, or current. We may make changes to the materials contained on its service at any time without notice. However, we do not make any commitment to update the materials.</p>

<h2>6. Links</h2>
<p>We have not reviewed all of the sites linked to our service and are not responsible for the contents of any such linked site. The inclusion of any link does not imply endorsement by us of the site. Use of any such linked website is at the user's own risk.</p>

<h2>7. Modifications</h2>
<p>We may revise these terms of service for its service at any time without notice. By using this service, you are agreeing to be bound by the then current version of these terms of service.</p>

<h2>8. Governing Law</h2>
<p>These terms and conditions are governed by and construed in accordance with the laws and you irrevocably submit to the exclusive jurisdiction of the courts in that location.</p>`

	now := apptime.NowTime()
	status := StatusPublished
	createdBy := "system"

	_, err = queries.CreateLegalDocument(ctx, db.CreateLegalDocumentParams{
		ID:           fmt.Sprintf("terms-%d", now.Unix()),
		DocumentType: "terms",
		Title:        "Terms and Conditions",
		Content:      &termsContent,
		Version:      1,
		Status:       &status,
		CreatedBy:    &createdBy,
	})
	if err != nil {
		return fmt.Errorf("failed to create terms document: %w", err)
	}

	// Create template Privacy Policy
	privacyContent := `<h2>1. Information We Collect</h2>
<p>We collect information you provide directly to us, such as when you create an account, submit a form, or communicate with us. The types of information we may collect include:</p>
<ul>
<li>Name and contact information</li>
<li>Account credentials</li>
<li>Payment information</li>
<li>Any other information you choose to provide</li>
</ul>

<h2>2. How We Use Your Information</h2>
<p>We use the information we collect to:</p>
<ul>
<li>Provide, maintain, and improve our services</li>
<li>Process transactions and send related information</li>
<li>Send technical notices, updates, security alerts, and support messages</li>
<li>Respond to your comments, questions, and requests</li>
<li>Communicate with you about products, services, offers, and events</li>
<li>Monitor and analyze trends, usage, and activities</li>
</ul>

<h2>3. Information Sharing</h2>
<p>We do not sell, trade, or otherwise transfer your personal information to third parties without your consent, except as described in this Privacy Policy. We may share information:</p>
<ul>
<li>With vendors, consultants, and other service providers who need access to such information to carry out work on our behalf</li>
<li>In response to a request for information if we believe disclosure is required by law</li>
<li>If we believe your actions are inconsistent with our user agreements or policies</li>
<li>To protect the rights, property, and safety of our organization and others</li>
</ul>

<h2>4. Data Security</h2>
<p>We take reasonable measures to help protect information about you from loss, theft, misuse, unauthorized access, disclosure, alteration, and destruction. However, no Internet transmission or electronic storage is completely secure, and we cannot guarantee absolute security.</p>

<h2>5. Data Retention</h2>
<p>We store the information we collect for as long as necessary for the purposes for which it was collected, to provide our services, resolve disputes, establish legal defenses, conduct audits, pursue legitimate business purposes, enforce our agreements, and comply with applicable laws.</p>

<h2>6. Your Rights</h2>
<p>You have the right to:</p>
<ul>
<li>Access the personal information we hold about you</li>
<li>Request correction of inaccurate information</li>
<li>Request deletion of your information</li>
<li>Object to our use of your information</li>
<li>Request portability of your information</li>
</ul>

<h2>7. Cookies</h2>
<p>We use cookies and similar tracking technologies to track activity on our service and hold certain information. You can instruct your browser to refuse all cookies or to indicate when a cookie is being sent.</p>

<h2>8. Children's Privacy</h2>
<p>Our service is not directed to individuals under the age of 13. We do not knowingly collect personal information from children under 13. If we become aware that we have collected personal information from a child under 13 without parental consent, we will take steps to delete that information.</p>

<h2>9. Changes to This Privacy Policy</h2>
<p>We may update our Privacy Policy from time to time. We will notify you of any changes by posting the new Privacy Policy on this page and updating the "Last Updated" date.</p>

<h2>10. Contact Us</h2>
<p>If you have any questions about this Privacy Policy, please contact us through the contact information provided on our website.</p>`

	_, err = queries.CreateLegalDocument(ctx, db.CreateLegalDocumentParams{
		ID:           fmt.Sprintf("privacy-%d", now.Unix()),
		DocumentType: "privacy",
		Title:        "Privacy Policy",
		Content:      &privacyContent,
		Version:      1,
		Status:       &status,
		CreatedBy:    &createdBy,
	})
	if err != nil {
		return fmt.Errorf("failed to create privacy document: %w", err)
	}

	return nil
}
